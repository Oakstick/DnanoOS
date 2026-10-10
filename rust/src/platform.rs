//! Small persistent key/value store: a settings file next to the user's other
//! game saves on desktop (Steam Cloud can sync that folder), localStorage on the web.
//!
//! On desktop each write goes to a scratch file first and is renamed over the settings file, and
//! the settings file it replaces is kept as `settings.txt.bak`; a settings file that is missing or
//! will not parse is read from that copy instead. `save_all` writes several keys in one go (the
//! run's snapshot: `game::persist`) and says when it could not.
//!
//! The game itself writes behind (`write_behind`): its writes go, in order, to a thread of their
//! own, so a save at a calm moment mid-fight no longer stalls the frame (3 to 10 ms of file work on
//! the game thread); a failure is told later (`take_write_failure`), a read first waits for the
//! writes before it, and the game waits for the last of them as it closes (`finish_writes`).

#[cfg(not(target_arch = "wasm32"))]
mod imp {
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::{Path, PathBuf};

    const APP_DIR: &str = "VoidHarvest";
    const FILE: &str = "settings.txt";

    /// %APPDATA%\VoidHarvest on Windows, the XDG data dir on Linux, Application Support on macOS.
    pub fn save_dir() -> Option<PathBuf> {
        #[cfg(test)]
        if let Some(dir) = DIR.with(|d| d.borrow().clone()) {
            return Some(dir);
        }
        if let Ok(dir) = std::env::var("VH_SAVE_DIR") {
            return Some(PathBuf::from(dir));
        }
        #[cfg(target_os = "windows")]
        let base = std::env::var_os("APPDATA").map(PathBuf::from);
        #[cfg(target_os = "macos")]
        let base = std::env::var_os("HOME").map(|h| PathBuf::from(h).join("Library/Application Support"));
        #[cfg(all(unix, not(target_os = "macos")))]
        let base = std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")));
        base.map(|b| b.join(APP_DIR))
    }

    /// The settings file's keys and values, and whether it is whole: every line has its `=`, and
    /// there is a line at all (a file torn or emptied; the game never writes an empty one).
    fn parse(text: &str) -> (BTreeMap<String, String>, bool) {
        let mut map = BTreeMap::new();
        let mut whole = true;
        for line in text.lines().filter(|l| !l.trim().is_empty()) {
            match line.split_once('=') {
                Some((k, v)) => {
                    map.insert(k.trim().to_string(), v.trim().to_string());
                }
                None => whole = false,
            }
        }
        let whole = whole && !map.is_empty();
        (map, whole)
    }

    /// What is saved, and whether it came whole from the settings file. A settings file missing
    /// or broken is read from its copy; with no good copy either, what can be read of it.
    fn read_state(dir: &Path) -> (BTreeMap<String, String>, bool) {
        // (A file that is not text at all reads as an empty, broken one.)
        let read = |name: &str| fs::read_to_string(dir.join(name)).ok().map(|t| parse(&t));
        let main = read(FILE);
        if let Some((map, true)) = main {
            return (map, true);
        }
        if let Some((map, true)) = read(&format!("{FILE}.bak")) {
            return (map, false);
        }
        (main.map(|m| m.0).unwrap_or_default(), false)
    }

    fn read_all() -> BTreeMap<String, String> {
        save_dir().map(|d| read_state(&d).0).unwrap_or_default()
    }

    pub fn load(key: &str) -> Option<String> {
        // (What is still being written is read as written.)
        finish_writes();
        read_all().remove(key)
    }

    /// The writer the game's writes go to when it writes behind: its queue, the writes not yet
    /// done, and the failures not yet told.
    /// One write behind: the save folder, and the keys and values.
    type Job = (PathBuf, Vec<(String, String)>);

    struct Behind {
        #[cfg_attr(test, allow(dead_code))]
        jobs: std::sync::Mutex<std::sync::mpsc::Sender<Job>>,
        busy: std::sync::Mutex<usize>,
        idle: std::sync::Condvar,
        failures: std::sync::Mutex<Vec<String>>,
    }

    static BEHIND: std::sync::OnceLock<Behind> = std::sync::OnceLock::new();

    /// From now on, writes go to a thread of their own, in order (the game, and the performance
    /// harness that measures it; tests and tools write in place).
    pub fn write_behind() {
        BEHIND.get_or_init(|| {
            let (tx, rx) = std::sync::mpsc::channel::<Job>();
            std::thread::spawn(move || {
                for (dir, entries) in rx {
                    let pairs: Vec<(&str, &str)> = entries.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
                    let result = write_all(&dir, &pairs);
                    if let Some(b) = BEHIND.get() {
                        if let Err(e) = result {
                            b.failures.lock().unwrap_or_else(|e| e.into_inner()).push(e);
                        }
                        let mut busy = b.busy.lock().unwrap_or_else(|e| e.into_inner());
                        *busy -= 1;
                        b.idle.notify_all();
                    }
                }
            });
            Behind {
                jobs: std::sync::Mutex::new(tx),
                busy: std::sync::Mutex::new(0),
                idle: std::sync::Condvar::new(),
                failures: std::sync::Mutex::new(Vec::new()),
            }
        });
    }

    /// Waits until every write handed to the writer is done (none when writing in place).
    pub fn finish_writes() {
        let Some(b) = BEHIND.get() else { return };
        let mut busy = b.busy.lock().unwrap_or_else(|e| e.into_inner());
        while *busy > 0 {
            busy = b.idle.wait(busy).unwrap_or_else(|e| e.into_inner());
        }
    }

    /// A write behind that failed since last asked, if any (what went wrong).
    pub fn take_write_failure() -> Option<String> {
        let b = BEHIND.get()?;
        let mut f = b.failures.lock().unwrap_or_else(|e| e.into_inner());
        (!f.is_empty()).then(|| f.remove(0))
    }

    /// Appends `line` to the log file `name` in the save folder, starting it with `header`.
    pub fn append_line(name: &str, header: &str, line: &str) {
        use std::io::Write;
        let Some(dir) = save_dir() else { return };
        let _ = fs::create_dir_all(&dir);
        let path = dir.join(name);
        let fresh = !path.exists();
        if let Ok(mut f) = fs::OpenOptions::new().create(true).append(true).open(path) {
            if fresh {
                let _ = writeln!(f, "{header}");
            }
            let _ = writeln!(f, "{line}");
        }
    }

    /// Writes `entries` into the settings file, all in one write. The file it replaces (if it
    /// was whole) becomes the copy.
    pub fn save_all(entries: &[(&str, &str)]) -> Result<(), String> {
        let Some(dir) = save_dir() else { return Ok(()) };
        #[cfg(not(test))]
        if let Some(b) = BEHIND.get() {
            // Behind: queued in order; a failure is told by `take_write_failure`.
            let owned = entries.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
            *b.busy.lock().unwrap_or_else(|e| e.into_inner()) += 1;
            if b.jobs.lock().unwrap_or_else(|e| e.into_inner()).send((dir.clone(), owned)).is_ok() {
                return Ok(());
            }
            *b.busy.lock().unwrap_or_else(|e| e.into_inner()) -= 1;
        }
        write_all(&dir, entries)
    }

    /// `entries` written into the settings file in `dir`, as `save_all` says.
    fn write_all(dir: &Path, entries: &[(&str, &str)]) -> Result<(), String> {
        let dir = dir.to_path_buf();
        let (mut map, whole) = read_state(&dir);
        for (k, v) in entries {
            map.insert(k.to_string(), v.to_string());
        }
        let text: String = map
            .iter()
            .map(|(k, v)| {
                format!(
                    "{k}={v}
"
                )
            })
            .collect();
        fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        #[cfg(test)]
        WRITES.with(|w| w.set(w.get() + 1));
        // Write then rename, so a crash mid-save never leaves a torn file.
        let tmp = dir.join(format!("{FILE}.tmp"));
        let main = dir.join(FILE);
        fs::write(&tmp, text).map_err(|e| format!("{}: {e}", tmp.display()))?;
        if whole {
            // (Should this fail, the new file still replaces the old one: only the copy is stale.)
            if let Err(e) = fs::rename(&main, dir.join(format!("{FILE}.bak"))) {
                eprintln!("save: could not keep the copy of {}: {e}", main.display());
            }
        }
        fs::rename(&tmp, &main).map_err(|e| format!("{}: {e}", main.display()))
    }

    pub fn save(key: &str, value: &str) {
        if let Err(e) = save_all(&[(key, value)]) {
            warn(&format!("could not save {key}: {e}"));
        }
    }

    /// A line for whoever reads the log (the console).
    pub fn warn(line: &str) {
        eprintln!("{line}");
    }

    #[cfg(test)]
    thread_local! {
        /// The save folder of this test's thread (tests run side by side, each on its own thread).
        static DIR: std::cell::RefCell<Option<PathBuf>> = const { std::cell::RefCell::new(None) };
        /// The settings files written on this thread.
        static WRITES: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
    }

    /// Saves on this test's thread go to `dir` (and nowhere else), whatever `VH_SAVE_DIR` says.
    #[cfg(test)]
    pub fn use_dir_on_this_thread(dir: PathBuf) {
        DIR.with(|d| *d.borrow_mut() = Some(dir));
    }

    /// How many times this test's thread has written the settings file.
    #[cfg(test)]
    pub fn writes_on_this_thread() -> u32 {
        WRITES.with(|w| w.get())
    }
}

#[cfg(target_arch = "wasm32")]
mod imp {
    fn storage() -> Option<web_sys::Storage> {
        web_sys::window()?.local_storage().ok()?
    }

    pub fn load(key: &str) -> Option<String> {
        storage()?.get_item(key).ok()?
    }

    /// Writes `entries`, one key after another (the page cannot be closed between them).
    pub fn save_all(entries: &[(&str, &str)]) -> Result<(), String> {
        let s = storage().ok_or("no local storage")?;
        for (k, v) in entries {
            s.set_item(k, v).map_err(|e| format!("{k}: {e:?}"))?;
        }
        Ok(())
    }

    pub fn save(key: &str, value: &str) {
        if let Err(e) = save_all(&[(key, value)]) {
            warn(&format!("could not save {key}: {e}"));
        }
    }

    /// (The web writes in place: local storage is quick.)
    pub fn write_behind() {}

    pub fn finish_writes() {}

    pub fn take_write_failure() -> Option<String> {
        None
    }

    /// A line for whoever reads the log (the browser's console).
    pub fn warn(line: &str) {
        web_sys::console::warn_1(&line.into());
    }

    /// Keeps the last lines of the log `name` in storage, under its header.
    pub fn append_line(name: &str, header: &str, line: &str) {
        const KEEP: usize = 200;
        let key = format!("vh.log.{name}");
        let old = load(&key).unwrap_or_default();
        let mut lines: Vec<&str> = old.lines().skip(1).collect();
        lines.push(line);
        let start = lines.len().saturating_sub(KEEP);
        save(&key, &format!("{header}\n{}", lines[start..].join("\n")));
    }
}

pub use imp::*;

/// Seconds since 1970, for stamping logs.
pub fn now_secs() -> u64 {
    web_time::SystemTime::now().duration_since(web_time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

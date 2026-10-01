//! Small persistent key/value store: a settings file next to the user's other
//! game saves on desktop (Steam Cloud can sync that folder), localStorage on the web.

#[cfg(not(target_arch = "wasm32"))]
mod imp {
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::PathBuf;

    const APP_DIR: &str = "VoidHarvest";
    const FILE: &str = "settings.txt";

    /// %APPDATA%\VoidHarvest on Windows, the XDG data dir on Linux, Application Support on macOS.
    pub fn save_dir() -> Option<PathBuf> {
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

    fn read_all() -> BTreeMap<String, String> {
        let mut map = BTreeMap::new();
        let Some(path) = save_dir().map(|d| d.join(FILE)) else { return map };
        if let Ok(text) = fs::read_to_string(path) {
            for line in text.lines() {
                if let Some((k, v)) = line.split_once('=') {
                    map.insert(k.trim().to_string(), v.trim().to_string());
                }
            }
        }
        map
    }

    pub fn load(key: &str) -> Option<String> {
        read_all().remove(key)
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

    pub fn save(key: &str, value: &str) {
        let Some(dir) = save_dir() else { return };
        let mut map = read_all();
        map.insert(key.to_string(), value.to_string());
        let text: String = map.iter().map(|(k, v)| format!("{k}={v}\n")).collect();
        let _ = fs::create_dir_all(&dir);
        // Write then rename, so a crash mid-save never leaves a torn file.
        let tmp = dir.join(format!("{FILE}.tmp"));
        if fs::write(&tmp, text).is_ok() {
            let _ = fs::rename(&tmp, dir.join(FILE));
        }
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

    pub fn save(key: &str, value: &str) {
        if let Some(s) = storage() {
            let _ = s.set_item(key, value);
        }
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

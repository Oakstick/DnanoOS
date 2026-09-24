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
}

pub use imp::*;

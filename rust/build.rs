//! Embeds the icon and version info into the game exe on Windows.
//! The icon is generated from the game's own voxel art: `AdventuresOfTheAncientBlades --write-icon assets/icon.ico`.

fn main() {
    // The build's identity for performance reports: the commit, and whether the tree had changes.
    println!("cargo:rerun-if-changed=../.git/HEAD");
    println!("cargo:rerun-if-changed=../.git/index");
    let git = |args: &[&str]| {
        std::process::Command::new("git")
            .args(args)
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
    };
    if let Some(hash) = git(&["rev-parse", "--short", "HEAD"]) {
        let dirty = git(&["status", "--porcelain", "--untracked-files=no"]).is_some_and(|s| !s.is_empty());
        println!("cargo:rustc-env=VH_BUILD={hash}{}", if dirty { "-modified" } else { "" });
    }
    println!("cargo:rerun-if-changed=assets/icon.ico");
    let windows = std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows");
    if !windows || !std::path::Path::new("assets/icon.ico").exists() {
        return;
    }
    let mut res = winresource::WindowsResource::new();
    res.set_icon("assets/icon.ico")
        .set("ProductName", "Adventures of the Ancient Blades")
        .set("FileDescription", "Adventures of the Ancient Blades")
        .set("LegalCopyright", "Copyright (c) 2026");
    if let Err(e) = res.compile() {
        println!("cargo:warning=icon not embedded: {e}");
    }
}

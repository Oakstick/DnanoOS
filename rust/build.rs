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
    if windows {
        copy_dxc();
    }
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

/// Puts Microsoft's DXC shader compiler (`dxcompiler.dll`, `dxil.dll`, redistributable) next to
/// the exe, from the newest Windows SDK installed: the game compiles its Direct3D 12 shaders with
/// it at start-up, many times faster than with the FXC built into Windows. Ship both with the
/// game; without them it falls back to FXC.
fn copy_dxc() {
    let Some(out) = std::env::var_os("OUT_DIR").map(std::path::PathBuf::from) else { return };
    // OUT_DIR is target/<profile>/build/<crate>/out: the exe goes in target/<profile>.
    let Some(exe_dir) = out.ancestors().nth(3) else { return };
    let kits = std::path::Path::new(r"C:\Program Files (x86)\Windows Kits\10\bin");
    let Ok(entries) = std::fs::read_dir(kits) else { return };
    let mut versions: Vec<_> = entries.flatten().map(|e| e.path()).filter(|p| p.join("x64").join("dxcompiler.dll").exists()).collect();
    versions.sort();
    let Some(sdk) = versions.last().map(|v| v.join("x64")) else {
        println!("cargo:warning=no Windows SDK DXC found: Direct3D 12 shaders will compile with the slow FXC");
        return;
    };
    for dll in ["dxcompiler.dll", "dxil.dll"] {
        let to = exe_dir.join(dll);
        if !to.exists() {
            if let Err(e) = std::fs::copy(sdk.join(dll), &to) {
                println!("cargo:warning=could not copy {dll}: {e}");
            }
        }
    }
}

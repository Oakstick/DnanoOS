//! Embeds the icon and version info into VoidHarvest.exe on Windows.
//! The icon is generated from the game's own voxel art: `VoidHarvest --write-icon assets/icon.ico`.

fn main() {
    println!("cargo:rerun-if-changed=assets/icon.ico");
    let windows = std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows");
    if !windows || !std::path::Path::new("assets/icon.ico").exists() {
        return;
    }
    let mut res = winresource::WindowsResource::new();
    res.set_icon("assets/icon.ico")
        .set("ProductName", "Void Harvest")
        .set("FileDescription", "Void Harvest")
        .set("LegalCopyright", "Copyright (c) 2026");
    if let Err(e) = res.compile() {
        println!("cargo:warning=icon not embedded: {e}");
    }
}

//! Embeds the icon and the version information in `tunneler.exe` on MSVC.

use std::env;
use std::io;

const ICON_PATH: &str = "../../app/src-tauri/icons/icon.ico";
const PRODUCT_NAME: &str = "Tunneler";

fn main() -> io::Result<()> {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={ICON_PATH}");
    if !targets_windows_msvc() {
        return Ok(());
    }
    let mut resource = winresource::WindowsResource::new();
    resource
        .set_icon(ICON_PATH)
        .set("ProductName", PRODUCT_NAME)
        .set(
            "FileDescription",
            &package_variable("CARGO_PKG_DESCRIPTION"),
        )
        .set("ProductVersion", &package_variable("CARGO_PKG_VERSION"))
        .set("FileVersion", &package_variable("CARGO_PKG_VERSION"));
    resource.compile()
}

fn targets_windows_msvc() -> bool {
    env::var("CARGO_CFG_TARGET_OS").is_ok_and(|target_os| target_os == "windows")
        && env::var("CARGO_CFG_TARGET_ENV").is_ok_and(|target_env| target_env == "msvc")
}

fn package_variable(name: &str) -> String {
    env::var(name).unwrap_or_default()
}

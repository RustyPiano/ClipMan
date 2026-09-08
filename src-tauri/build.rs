fn main() {
    // Set macOS to be a background/menu bar only app (no Dock icon)
    #[cfg(target_os = "macos")]
    {
        println!("cargo:rustc-env=MACOSX_DEPLOYMENT_TARGET=13.3");
    }

    tauri_build::build()
}

fn main() {
    #[cfg(target_os = "macos")]
    {
        println!("cargo:rustc-env=MACOSX_DEPLOYMENT_TARGET=13.3");
    }

    tauri_build::build()
}

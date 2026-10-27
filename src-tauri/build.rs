fn main() {
    println!("cargo:rerun-if-env-changed=AWAKE_WINDOWS_TEST_RESOURCES");
    tauri_build::build();

    let windows_msvc = std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        && std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc");
    if windows_msvc && std::env::var("AWAKE_WINDOWS_TEST_RESOURCES").as_deref() == Ok("1") {
        // tauri-build 2.7 links resources into app binaries, but not lib test
        // executables. Reuse its exact manifest/icon resource for `test --lib`.
        // Keep this opt-in: all-target builds would link bin resources twice.
        // https://github.com/tauri-apps/tauri/issues/14580
        let resource = std::path::PathBuf::from(std::env::var_os("OUT_DIR").expect("OUT_DIR"))
            .join("resource.lib");
        assert!(
            resource.is_file(),
            "Missing Windows test resource: {}",
            resource.display()
        );
        println!("cargo:rustc-link-arg={}", resource.display());
    }
}

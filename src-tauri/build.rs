fn main() {
    println!("cargo:rerun-if-changed=swift/TuckReader.swift");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        std::fs::create_dir_all("resources").expect("create reader directory");
        let status = std::process::Command::new("swiftc")
            .args([
                "-O",
                "swift/TuckReader.swift",
                "-o",
                "resources/tuck-reader",
            ])
            .status()
            .expect("Swift compiler is required for local OCR");
        assert!(
            status.success(),
            "Could not compile the local document reader"
        );
    }
    tauri_build::build()
}

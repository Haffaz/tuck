/// True when `output` exists and is newer than every source. Helpers in `resources/` are
/// bundled, so tauri-build reruns this script when they change; recompiling them on every
/// run would then make `tauri dev` rebuild forever.
fn up_to_date(output: &str, sources: &[&str]) -> bool {
    let modified = |path: &str| std::fs::metadata(path).and_then(|m| m.modified()).ok();
    match modified(output) {
        Some(built) => sources
            .iter()
            .all(|source| modified(source).is_some_and(|changed| changed <= built)),
        None => false,
    }
}

fn main() {
    println!("cargo:rerun-if-changed=swift/TuckReader.swift");
    println!("cargo:rerun-if-changed=swift/TuckClues.swift");
    const READER_SOURCES: &[&str] = &["swift/TuckReader.swift", "swift/TuckClues.swift"];
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos")
        && !up_to_date("resources/tuck-reader", READER_SOURCES)
    {
        std::fs::create_dir_all("resources").expect("create reader directory");
        let status = std::process::Command::new("swiftc")
            .args(["-parse-as-library", "-O"])
            .args(READER_SOURCES)
            .args(["-o", "resources/tuck-reader"])
            .status()
            .expect("Swift compiler is required for local OCR");
        assert!(
            status.success(),
            "Could not compile the local document reader"
        );
    }
    println!("cargo:rerun-if-changed=swift/TuckConnections.swift");
    println!("cargo:rerun-if-changed=swift/TuckLocalTransport.swift");
    println!("cargo:rerun-if-changed=swift/TuckApple.swift");
    println!("cargo:rerun-if-changed=swift/TuckCards.swift");
    println!("cargo:rerun-if-changed=swift/TuckModelSupport.swift");
    const HELPER_SOURCES: &[&str] = &[
        "swift/TuckApple.swift",
        "swift/TuckLocalTransport.swift",
        "swift/TuckConnections.swift",
        "swift/TuckModelSupport.swift",
        "swift/TuckCards.swift",
    ];
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos")
        && !up_to_date("resources/tuck-apple", HELPER_SOURCES)
    {
        let status = std::process::Command::new("swiftc")
            .args(["-parse-as-library", "-O"])
            .args(HELPER_SOURCES)
            .args(["-o", "resources/tuck-apple"])
            .status()
            .expect("Swift compiler is required for Apple Intelligence");
        assert!(
            status.success(),
            "Could not compile the Apple Intelligence helper"
        );
    }
    println!("cargo:rerun-if-changed=swift/TuckMac.swift");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        let output = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
        let status = std::process::Command::new("swiftc")
            .args([
                "-parse-as-library",
                "-emit-library",
                "-static",
                "-O",
                "swift/TuckMac.swift",
                "-o",
            ])
            .arg(output.join("libtuck_mac.a"))
            .status()
            .expect("Swift compiler is required for native Mac integration");
        assert!(status.success(), "Could not compile native Mac integration");
        println!("cargo:rustc-link-search=native={}", output.display());
        println!("cargo:rustc-link-lib=static=tuck_mac");
        println!("cargo:rustc-link-lib=framework=AppKit");
        println!("cargo:rustc-link-lib=framework=Foundation");
        println!("cargo:rustc-link-lib=framework=QuickLookUI");
        println!("cargo:rustc-link-search=native=/usr/lib/swift");
        println!("cargo:rustc-link-arg=-Wl,-rpath,/usr/lib/swift");
    }
    tauri_build::build()
}

fn main() {
    // Re-embed sqlx migrations when a file is added or changed (`sqlx::migrate!` reads them at compile time).
    println!("cargo:rerun-if-changed=migrations");
    translate_helper();
    tauri_build::build()
}

/// Apple's on-device translation (D-046): `helpers/translate.swift` built for the target as
/// `binaries/newdawn-translate-<triple>`, which Tauri bundles beside the app (`externalBin`) and
/// copies beside the dev binary. The deployment target matches `minimumSystemVersion`; the
/// Translation framework is weak-linked (macOS 15+), the helper checks the rest itself.
fn translate_helper() {
    println!("cargo:rerun-if-changed=helpers/translate.swift");
    let target = std::env::var("TARGET").expect("cargo sets TARGET");
    let arch = match target.split('-').next() {
        Some("aarch64") => "arm64",
        Some(other) => other,
        None => panic!("unexpected target {target}"),
    };
    std::fs::create_dir_all("binaries").expect("creating src-tauri/binaries");
    let out = format!("binaries/newdawn-translate-{target}");
    let status = std::process::Command::new("xcrun")
        .args(["swiftc", "-parse-as-library", "-O", "-target"])
        .arg(format!("{arch}-apple-macos14.2"))
        .args(["-Xlinker", "-weak_framework", "-Xlinker", "Translation"])
        .args(["helpers/translate.swift", "-o", &out])
        .status()
        .expect("running swiftc (Xcode or its command line tools)");
    assert!(status.success(), "building helpers/translate.swift failed");
}

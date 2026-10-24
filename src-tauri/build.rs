fn main() {
    // Re-embed sqlx migrations when a file is added or changed (`sqlx::migrate!` reads them at compile time).
    println!("cargo:rerun-if-changed=migrations");
    tauri_build::build()
}

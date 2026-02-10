// Rebuild when locale files change so rust_i18n re-embeds translations.
fn main() {
    println!("cargo:rerun-if-changed=locales");
    if let Ok(entries) = std::fs::read_dir("locales") {
        for entry in entries.flatten() {
            if entry.path().extension().is_some_and(|ext| ext == "yml") {
                println!("cargo:rerun-if-changed={}", entry.path().display());
            }
        }
    }
}

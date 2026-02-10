// Rebuild when locale files change so rust_i18n re-embeds translations.
fn main() {
    println!("cargo:rerun-if-changed=locales/en.yml");
    println!("cargo:rerun-if-changed=locales/es.yml");
    println!("cargo:rerun-if-changed=locales/zh-CN.yml");
}

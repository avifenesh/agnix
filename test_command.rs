fn main() {
    // Test edge case: "npm install-something" - should this match?
    let raw = "npm install-something";
    
    // Current logic checks:
    if raw.contains(" install") {
        println!("Matches ' install' (with space)");
    } else {
        println!("Does NOT match ' install'");
    }
    
    // Test "npm i" (short form) - edge case
    let raw2 = "npm i";
    if raw2.contains(" i ") {
        println!("'npm i' matches ' i ' (with spaces)");
    } else {
        println!("'npm i' does NOT match ' i ' (needs space after)");
    }
    
    // Test "npm install" (no trailing text)
    let raw3 = "npm install";
    if raw3.contains(" install") {
        println!("'npm install' matches ' install'");
    }
}

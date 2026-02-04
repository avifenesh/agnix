use regex::Regex;

fn main() {
    let pattern = Regex::new(r"(?m)(?:^|\s|`)((?:npm|pnpm|yarn|bun)\s+(?:install|i|add|build|test|run|exec|ci)\b[^\n`]*)").unwrap();
    
    let tests = vec![
        "Run `npm i` to install",
        "Run `npm install` to install",
        "Use npm i",
        "Try npm i\n",
    ];
    
    for test in tests {
        println!("\nTest: '{}'", test.escape_default());
        for caps in pattern.captures_iter(test) {
            if let Some(m) = caps.get(1) {
                let raw = m.as_str();
                println!("  Captured: '{}'", raw);
                println!("  Contains ' i ': {}", raw.contains(" i "));
                println!("  Trimmed: '{}'", raw.trim());
            }
        }
    }
}

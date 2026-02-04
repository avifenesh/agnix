fn main() {
    // Check if "npm install" at end of line works
    let tests = vec![
        ("npm install", " install"),
        ("npm i ", " i "),
        ("npm i", " i "),
        ("yarn add", " add"),
        ("pnpm ci", " ci"),
    ];
    
    for (cmd, pattern) in tests {
        let matches = cmd.contains(pattern);
        println!("'{}' contains '{}': {}", cmd, pattern, matches);
    }
}

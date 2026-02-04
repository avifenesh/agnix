fn main() {
    let line = "allowed-tools: Read Write Bash Read";
    let remainder = &line[14..]; // After "allowed-tools:"
    let remainder_lower = remainder.to_lowercase();
    
    println!("Remainder: '{}'", remainder);
    
    // This will only find the FIRST occurrence
    if let Some(pos) = remainder_lower.find("read") {
        println!("Found 'read' at position: {}", pos);
        println!("This means we only detect Read once, not twice!");
    }
}

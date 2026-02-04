fn is_word_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn main() {
    // Test case: "Bashé" where é is UTF-8 multi-byte character
    let text = "Bashé";
    let text_lower = text.to_lowercase();
    let text_bytes = text_lower.as_bytes();
    
    println!("Text: '{}'", text);
    println!("Lower: '{}'", text_lower);
    println!("Bytes: {:?}", text_bytes);
    println!("Length: {}", text_bytes.len());
    
    // Find "bash" at position 0
    if let Some(pos) = text_lower.find("bash") {
        println!("\nFound 'bash' at pos: {}", pos);
        let after_pos = pos + "bash".len(); // = 4
        println!("After pos: {}", after_pos);
        
        if after_pos < text_bytes.len() {
            println!("Byte at position {}: {} ('{}')", after_pos, text_bytes[after_pos], text_bytes[after_pos] as char);
            println!("Is word char?: {}", is_word_char(text_bytes[after_pos]));
        }
    }
}

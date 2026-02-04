fn main() {
    // Simulate what happens after regex capture
    let captured = "npm i"; // This is what regex captures
    
    // Current detection logic
    let matches_space_i_space = captured.contains(" i ");
    let matches_space_install = captured.contains(" install");
    let matches_space_add = captured.contains(" add");
    let matches_space_ci = captured.contains(" ci");
    
    println!("Raw captured: '{}'", captured);
    println!("Matches ' i ': {}", matches_space_i_space);
    println!("Matches ' install': {}", matches_space_install);
    println!("Matches ' add': {}", matches_space_add);
    println!("Matches ' ci': {}", matches_space_ci);
    
    // The issue: none of these match!
    if matches_space_i_space || matches_space_install || matches_space_add || matches_space_ci {
        println!("\nCommand type: Install");
    } else {
        println!("\nCommand type: Other (BUG!)");
    }
    
    // Better approach - check if it ends with the command
    println!("\n=== Better approach ===");
    let ends_with_i = captured.ends_with(" i");
    println!("Ends with ' i': {}", ends_with_i);
}

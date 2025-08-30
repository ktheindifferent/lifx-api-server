use std::fs;
use std::path::Path;

#[test]
fn no_println_in_production_code() {
    let src_files = vec![
        "src/lib.rs",
        "src/main.rs",
        "src/set_states.rs",
    ];
    
    for file_path in src_files {
        let contents = fs::read_to_string(file_path)
            .expect(&format!("Failed to read file: {}", file_path));
        
        // Check for println! statements (excluding comments)
        for (line_num, line) in contents.lines().enumerate() {
            let trimmed = line.trim();
            // Skip commented lines
            if trimmed.starts_with("//") {
                continue;
            }
            
            // Check for println! not in comments
            if line.contains("println!") && !line.trim_start().starts_with("//") {
                panic!(
                    "Found println! statement in {} at line {}: {}",
                    file_path,
                    line_num + 1,
                    line
                );
            }
            
            // Check for print! not in comments  
            if line.contains("print!") && !line.contains("eprintln!") && !line.contains("println!") 
                && !line.trim_start().starts_with("//") {
                panic!(
                    "Found print! statement in {} at line {}: {}",
                    file_path,
                    line_num + 1,
                    line
                );
            }
        }
    }
}

#[test]
fn verify_logging_initialized() {
    // Verify that env_logger is initialized in main.rs
    let main_contents = fs::read_to_string("src/main.rs")
        .expect("Failed to read main.rs");
    
    assert!(
        main_contents.contains("env_logger::init"),
        "env_logger::init() not found in main.rs - logging may not be initialized"
    );
    
    assert!(
        main_contents.contains("use log:"),
        "log crate not imported in main.rs"
    );
}

#[test]
fn verify_log_dependencies() {
    // Verify that log and env_logger are in Cargo.toml
    let cargo_contents = fs::read_to_string("Cargo.toml")
        .expect("Failed to read Cargo.toml");
    
    assert!(
        cargo_contents.contains("log ="),
        "log dependency not found in Cargo.toml"
    );
    
    assert!(
        cargo_contents.contains("env_logger ="),
        "env_logger dependency not found in Cargo.toml"
    );
}
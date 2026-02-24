use std::path::Path;

fn main() {
    let db_path = "billing-system.db";

    if !Path::new(db_path).exists() {
        println!("Database not found at {}", db_path);
        // Check if exists in parent directory
        let alt_path = "../billing-system.db";
        if Path::new(alt_path).exists() {
            check_db(alt_path);
        } else {
            println!("Trying with sqlite3 to read database...");
        }
        return;
    }

    check_db(db_path);
}

fn check_db(db_path: &str) {
    // Use rusqlite if available, otherwise print info
    println!("Checking database at: {}", db_path);
    println!("Install sqlite3 or use rusqlite to check API key format");
}

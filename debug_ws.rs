//! Debug WebSocket messages to understand bid/ask format

use std::io::Read;

fn main() {
    println!("=== Debugging WebSocket Message Format ===");
    println!("Reading from stdin (pipe WebSocket output here)");
    println!();

    let mut buffer = vec![0u8; 1024 * 1024];
    let mut total = 0;

    loop {
        let n = match std::io::stdin().read(&mut buffer[total..]) {
            Ok(0) => break,
            Ok(n) => n,
            Err(_) => break,
        };
        total += n;

        // Print raw message
        if let Ok(msg) = std::str::from_utf8(&buffer[..total]) {
            println!("RAW MESSAGE:");
            println!("{}", msg.chars().take(500).collect::<String>());
            println!("...");
            println!();

            // Look for bids/asks
            if msg.contains("\"bids\"") {
                println!("Found bids section!");
            }
            if msg.contains("\"asks\"") {
                println!("Found asks section!");
            }
            if msg.contains("\"price_changes\"") {
                println!("Found price_changes array!");
            }

            // Look for asset_id structure
            if msg.contains("\"asset_id\"") {
                println!("Found asset_id field - this is the new format!");
            }

            break;
        }
    }
}
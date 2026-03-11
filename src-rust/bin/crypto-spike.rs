// Crypto compatibility spike - test Rust's ability to decrypt Go-generated age ciphertexts
//
// Usage:
//   cargo run --bin crypto-spike -- test-decrypt <path-to-master.key> <passphrase>
//   cargo run --bin crypto-spike -- decrypt-provider <path-to-database.json> <master-key-hex>
//
// This is the Phase 0.1 crypto spike to validate whether Rust can decrypt Go-generated data.

use age::{Decryptor, scrypt};
use std::env;
use std::fs;
use std::io::Read;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        eprintln!("Crypto Compatibility Spike - Phase 0.1");
        eprintln!("========================================");
        eprintln!();
        eprintln!("Usage:");
        eprintln!("  crypto-spike test-decrypt <path-to-master.key> <passphrase>");
        eprintln!("  crypto-spike decrypt-provider <path-to-database.json> <master-key-hex>");
        eprintln!();
        eprintln!(
            "This is the Phase 0.1 crypto spike to validate whether Rust can decrypt Go-generated data."
        );
        std::process::exit(1);
    }

    let command = &args[1];

    match command.as_str() {
        "test-decrypt" => {
            if args.len() < 4 {
                eprintln!("Error: test-decrypt requires <path-to-master.key> and <passphrase>");
                std::process::exit(1);
            }
            test_decrypt_master_key(&args[2], &args[3]);
        }
        "decrypt-provider" => {
            if args.len() < 4 {
                eprintln!(
                    "Error: decrypt-provider requires <path-to-database.json> and <master-key-hex>"
                );
                std::process::exit(1);
            }
            decrypt_provider_ciphers(&args[2], &args[3]);
        }
        _ => {
            eprintln!("Unknown command: {}", command);
            std::process::exit(1);
        }
    }
}

fn test_decrypt_master_key(path: &str, passphrase: &str) {
    println!("=== Crypto Compatibility Spike ===");
    println!("Testing decryption of Go-generated master.key");
    println!("File: {}", path);
    println!("Passphrase: {}", passphrase);
    println!();

    // Read the encrypted master.key file
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("ERROR: Failed to read master.key: {}", e);
            std::process::exit(1);
        }
    };

    println!("✓ Read master.key file ({} chars)", content.len());
    println!();

    // Try to decrypt using age scrypt
    println!("Attempting decryption with Rust age crate...");

    match decrypt_with_scrypt(&content, passphrase) {
        Ok(master_key) => {
            println!("✓ SUCCESS: Decrypted master key!");
            println!("Master key (hex): {}", hex::encode(&master_key));
            println!("Master key length: {} bytes", master_key.len());
            println!();
            println!("This confirms Rust can decrypt Go-generated age ciphertexts.");
            println!("Next: Test decryption of provider ciphers in database.json");
        }
        Err(e) => {
            eprintln!("✗ FAILED: Could not decrypt master.key");
            eprintln!("Error: {}", e);
            eprintln!();
            eprintln!(
                "This indicates a compatibility issue between Go and Rust age implementations."
            );
            eprintln!("Possible causes:");
            eprintln!("  1. Go string-to-bytes conversion differs from Rust");
            eprintln!("  2. age scrypt parameters differ");
            eprintln!("  3. Binary data in passphrase not handled identically");
            std::process::exit(1);
        }
    }
}

fn decrypt_with_scrypt(
    ciphertext: &str,
    passphrase: &str,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    // Create a scrypt identity for decryption
    let scrypt_identity = scrypt::Identity::new(passphrase.into());

    // Try to decrypt
    let decryptor = Decryptor::new(ciphertext.as_bytes())?;

    // Decrypt with the scrypt identity
    let mut plaintext = Vec::new();
    {
        let mut reader =
            decryptor.decrypt(&mut std::iter::once(&scrypt_identity as &dyn age::Identity))?;
        reader.read_to_end(&mut plaintext)?;
    }

    Ok(plaintext)
}

fn decrypt_provider_ciphers(database_path: &str, master_key_hex: &str) {
    println!("=== Decrypt Provider Ciphers ===");
    println!("Database: {}", database_path);
    println!(
        "Master key (hex): {}...",
        &master_key_hex[..min(16, master_key_hex.len())]
    );
    println!();

    // Parse master key from hex
    let master_key = match hex::decode(master_key_hex) {
        Ok(key) => key,
        Err(e) => {
            eprintln!("ERROR: Invalid master key hex: {}", e);
            std::process::exit(1);
        }
    };

    println!("✓ Decoded master key ({} bytes)", master_key.len());

    // Read database.json
    let content = match fs::read_to_string(database_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("ERROR: Failed to read database.json: {}", e);
            std::process::exit(1);
        }
    };

    // Parse JSON
    let database: serde_json::Value = match serde_json::from_str(&content) {
        Ok(db) => db,
        Err(e) => {
            eprintln!("ERROR: Failed to parse database.json: {}", e);
            std::process::exit(1);
        }
    };

    let providers = database["providers"].as_array().unwrap();
    println!("✓ Found {} provider entries", providers.len());
    println!();

    // Decrypt each provider
    for (i, provider) in providers.iter().enumerate() {
        let provider_name = provider["provider"].as_str().unwrap();
        let cipher = provider["cipher"].as_str().unwrap();

        println!("Provider {}: {}", i + 1, provider_name);

        match decrypt_with_scrypt(cipher, &String::from_utf8_lossy(&master_key)) {
            Ok(plaintext) => {
                let api_key = String::from_utf8_lossy(&plaintext);
                println!("  ✓ Decrypted API key: {}", api_key);
            }
            Err(e) => {
                eprintln!("  ✗ Failed to decrypt: {}", e);
            }
        }
    }
}

fn min(a: usize, b: usize) -> usize {
    if a < b { a } else { b }
}

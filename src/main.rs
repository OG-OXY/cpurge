use std::process::Command;
use std::path::{Path, PathBuf};
use std::collections::HashSet;
use std::fs;

#[derive(Debug)]
struct DiskItem {
    path: String,
    size_bytes: u64,
}

// 1. Space Hog Scanner (From your original.rs logic)
fn scan_directory(path: &Path, list: &mut Vec<DiskItem>) -> u64 {
    let mut total_size = 0;

    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let metadata = match entry.metadata() {
                Ok(data) => data,
                Err(_) => continue, // Skip files with permission errors
            };
            
            let entry_path = entry.path();

            if metadata.is_dir() {
                let dir_size = scan_directory(&entry_path, list);
                total_size += dir_size;

                list.push(DiskItem {
                    path: entry_path.display().to_string(),
                    size_bytes: dir_size,
                });
            } else {
                total_size += metadata.len();
            }
        }
    }

    total_size
}

// 2. Fetch Whitelist via Nix Eval (From your main.rs logic)
fn get_nix_whitelist() -> Result<HashSet<PathBuf>, Box<dyn std::error::Error>> {
    let output = Command::new("nix")
        .args(&[
            "eval",
            "--json",
            ".#homeConfigurations.ty.config.xdg.configFile",
        ])
        .output()?;

    if !output.status.success() {
        return Err(format!("nix eval failed: {}", String::from_utf8_lossy(&output.stderr)).into());
    }

    let configs: serde_json::Map<String, serde_json::Value> = serde_json::from_slice(&output.stdout)?;
    
    let home = dirs::home_dir().ok_or("Could not find home directory")?;
    let config_dir = home.join(".config");
    
    let mut whitelist = HashSet::new();
    for key in configs.keys() {
        whitelist.insert(config_dir.join(key));
    }

    Ok(whitelist)
}

// 3. Audit Config Directory against Nix Whitelist
fn audit_config_dir(whitelist: &HashSet<PathBuf>, dry_run: bool) -> std::io::Result<()> {
    let home = dirs::home_dir().unwrap();
    let config_dir = home.join(".config");

    if !config_dir.exists() {
        println!("Config directory does not exist: {}", config_dir.display());
        return Ok(());
    }

    for entry in walkdir::WalkDir::new(&config_dir).min_depth(1).max_depth(3) {
        let entry = entry?;
        let path = entry.path().to_path_buf();

        let is_allowed = whitelist.iter().any(|w| path == *w || path.starts_with(w));

        if !is_allowed {
            if dry_run {
                println!("[DRY RUN] Unmanaged (Candidate for purge): {}", path.display());
            } else {
                println!("Purging unmanaged: {}", path.display());
                if path.is_dir() {
                    fs::remove_dir_all(&path)?;
                } else {
                    fs::remove_file(&path)?;
                }
            }
        } else {
            println!("[MANAGED]: {}", path.display());
        }
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // --- PART 1: Space Hog Analysis ---
    let target_dir = ".";
    println!("=== Scanning {} for Space Hogs ===", target_dir);

    let path = Path::new(target_dir);
    let mut items: Vec<DiskItem> = Vec::new();
    let total_size = scan_directory(path, &mut items);

    items.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));

    println!("\nTop Space Hogs:");
    println!("{:<10} | {}", "Size (MB)", "Path");
    println!("--------------------------------------------------");
    for item in items.iter().take(10) {
        let mb = item.size_bytes / 1024 / 1024;
        if mb > 0 {
            println!("{:<10} | {}", mb, item.path);
        }
    }
    println!("\nTotal space scanned: {} MB\n", total_size / 1024 / 1024);

    // --- PART 2: Nix Whitelist Audit ---
    println!("=== Auditing ~/.config against Nix Whitelist ===");
    let whitelist = match get_nix_whitelist() {
        Ok(w) => {
            println!("Successfully loaded {} items from Nix configuration.", w.len());
            w
        }
        Err(e) => {
            eprintln!("Warning: Could not fetch Nix whitelist: {}", e);
            HashSet::new()
        }
    };

    // Set to 'true' so it only prints out what it *would* delete. 
    // Change to 'false' only when you are 100% ready to actually purge.
    let dry_run = true; 
    
    println!("\nRunning Audit (Dry Run: {}):", dry_run);
    println!("--------------------------------------------------");
    audit_config_dir(&whitelist, dry_run)?;

    Ok(())
}

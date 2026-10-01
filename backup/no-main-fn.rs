use std::process::Command;
use std::path::{Path, PathBuf};
use std::collections::HashSet;
use std::fs;

fn get_nix_whitelist() -> Result<HashSet<PathBuf>, Box<dyn std::error::Error>> {
    // Run nix eval to get the JSON list of managed config files
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

    // Parse the JSON keys (Home Manager config names)
    let configs: serde_json::Map<String, serde_json::Value> = serde_json::from_slice(&output.stdout)?;
    
    let home = dirs::home_dir().ok_or("Could not find home directory")?;
    let config_dir = home.join(".config");
    
    let mut whitelist = HashSet::new();
    for key in configs.keys() {
        // key might be something like "hypr/hyprland.conf" or just "hypr"
        whitelist.insert(config_dir.join(key));
    }

    Ok(whitelist)
}

fn audit_config_dir(whitelist: &HashSet<PathBuf>, dry_run: bool) -> std::io::Result<()> {
    let home = dirs::home_dir().unwrap();
    let config_dir = home.join(".config");

    for entry in walkdir::WalkDir::new(&config_dir).min_depth(1).max_depth(3) {
        let entry = entry?;
        let path = entry.path().to_path_buf();

        // Check if path is in whitelist or is a descendant of a whitelisted directory
        let is_allowed = whitelist.iter().any(|w| path == *w || path.starts_with(w));

        if !is_allowed {
            if dry_run {
                println!("[DRY RUN] Unmanaged: {}", path.display());
            } else {
                println!("Removing unmanaged: {}", path.display());
                if path.is_dir() {
                    fs::remove_dir_all(&path)?;
                } else {
                    fs::remove_file(&path)?;
                }
            }
        }
    }
    Ok(())
}

use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, Write};
use std::path::Path;
use std::collections::HashSet;

#[derive(Debug)]
struct DiskItem {
    path: String,
    size_bytes: u64,
}

// 1. Scan and generate a manifest file of what *should* be managed/tracked
fn generate_manifest(target_dir: &Path, manifest_path: &str) -> io::Result<()> {
    let mut file = File::create(manifest_path)?;
    println!("Generating manifest at {}...", manifest_path);

    // For demonstration, let's recursively list paths and write them to the manifest
    let mut entries_to_write = Vec::new();
    collect_paths(target_dir, &mut entries_to_write)?;

    for path in entries_to_write {
        writeln!(file, "{}", path)?;
    }

    Ok(())
}

// Helper to collect paths recursively for the manifest
fn collect_paths(path: &Path, list: &mut Vec<String>) -> io::Result<()> {
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let entry_path = entry.path();
            list.push(entry_path.display().to_string());
            if entry_path.is_dir() {
                collect_paths(&entry_path, list)?;
            }
        }
    }
    Ok(())
}

// 2. Read and parse the manifest back into a HashSet for O(1) lookups
fn load_manifest(manifest_path: &str) -> io::Result<HashSet<String>> {
    let file = File::open(manifest_path)?;
    let reader = BufReader::new(file);
    let mut declared_paths = HashSet::new();

    for line in reader.lines() {
        let path = line?;
        if !path.trim().is_empty() {
            declared_paths.insert(path);
        }
    }

    Ok(declared_paths)
}

// 3. Dry-run comparison: check live files against the manifest
fn audit_unmanaged_files(path: &Path, declared_paths: &HashSet<String>) {
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let entry_path = entry.path();
            let path_str = entry_path.display().to_string();

            if entry_path.is_dir() {
                audit_unmanaged_files(&entry_path, declared_paths);
            }

            // Check if the file/folder exists in our manifest
            if !declared_paths.contains(&path_str) {
                println!("[UNMANAGED / CANDIDATE FOR PURGE]: {}", path_str);
            } else {
                println!("[MANAGED]: {}", path_str);
            }
        }
    }
}

fn main() -> io::Result<()> {
    let target_dir = ".";
    let manifest_file = "cpurge_manifest.txt";

    // Step 1: Build the manifest from the target configuration/directory
    generate_manifest(Path::new(target_dir), manifest_file)?;

    // Step 2: Read it back and parse it
    let declared_paths = load_manifest(manifest_file)?;
    println!("\nSuccessfully loaded {} entries from manifest.", declared_paths.len());

    // Step 3: Run the dry-run audit parser to verify matching logic
    println!("\nRunning Dry-Run Audit:");
    println!("--------------------------------------------------");
    audit_unmanaged_files(Path::new(target_dir), &declared_paths);

    Ok(())
}

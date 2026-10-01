use std::fs;
use std::path::Path;

// A simple struct to hold path info and its total size in bytes
#[derive(Debug)]
struct DiskItem {
    path: String,
    size_bytes: u64,
}

fn scan_directory(path: &Path, list:&mut Vec<DiskItem>) -> u64 {
    let mut total_size = 0;
    let mut _sub_dir_size = 0;

    // Read directory entries safely using a match statement to handle permission errors
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let metadata = match entry.metadata() {
                Ok(data) => data,
                Err(_) => continue, // Skip files we can't read permissions for
            };
            
            let entry_path = entry.path();

            if metadata.is_dir() {
                // Recursively call for subdirectories
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

fn main() {
    let target_dir = "."; // For now, scan the current directory
    println!("Scanning {} for space hogs...", target_dir);

    let path = Path::new(target_dir);
    let mut items: Vec<DiskItem> = Vec::new();

    let total_size = scan_directory(path, &mut items);

    items.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));

    println!("\nTop Space Hogs:");
    println!("{:<10} | {}", "Size (MB)", "Path");
    println!("--------------------------------------------------");
    // Print out the top 10 biggest directories
    for item in items.iter().take(10) {
        let mb = item.size_bytes / 1024 / 1024;
        if mb > 0 {
            println!("{:<10} | {}", mb, item.path);
        }
    }
    println!("\nTotal space scanned: {} MB", total_size / 1024 / 1024);
}

use std::{fs, path::Path};

pub fn longest_common_prefix (strings: &[String]) -> String {
    
    if strings.is_empty() {
        return String::new();
    }

    let mut prefix = strings[0].clone();

    for s in &strings[1..] {
        while !s.starts_with(&prefix) {
            prefix.pop();

            if prefix.is_empty() {
                break;
            }
        }
    }

    prefix
}

pub fn get_file_matches (prefix: &str) -> Vec<String> {
    let mut matches = Vec::new();

    let (dir, partial) = match prefix.rfind('/') {
        Some(idx) => (&prefix[..idx + 1], &prefix[idx + 1..]),
        None => ("./", prefix)
    };

    let search_dir = Path::new(dir);

    if let Ok(entries) = fs::read_dir(search_dir) {
        for entry in entries.flatten() {
            let file_name = entry.file_name();

            if let Some(name) = file_name.to_str() { 
                if name.starts_with(partial) {
                    let full_path = format!("{}{}", dir, name);
                    let cleaned = full_path.strip_prefix("./").unwrap_or(&full_path);
                    matches.push(cleaned.to_string());
                }
            }
        }
    }
    matches
}
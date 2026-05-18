use std::fs;

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

    if let Ok(entries) = fs::read_dir(".") {
        for entry in entries.flatten() {
            let file_name = entry.file_name();

            if let Some(name) = file_name.to_str() {
                if name.starts_with(prefix) {
                    matches.push(name.to_string());
                }
            }
        }
    }
    matches
}
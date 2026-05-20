use std::{fs, path::Path};

pub fn longest_common_prefix(strings: &[String]) -> String {
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

pub fn get_file_matches(prefix: &str) -> Vec<String> {
    let mut matches = Vec::new();

    let (dir, partial) = match prefix.rfind('/') {
        Some(idx) => (&prefix[..idx + 1], &prefix[idx + 1..]),
        None => ("./", prefix),
    };

    if let Ok(entries) = fs::read_dir(Path::new(dir)) {
        for entry in entries.flatten() {
            let file_name = entry.file_name();
            let Some(name) = file_name.to_str() else {
                continue;
            };

            if !name.starts_with(partial) {
                continue;
            }

            let full_path = format!("{}{}", dir, name);
            let display = strip_dot_slash(&full_path);

            if Path::new(&full_path).is_dir() {
                matches.push(format!("{}/", display));
            } else {
                matches.push(display);
            }
        }
    }

    matches
}

fn strip_dot_slash(s: &str) -> String {
    s.strip_prefix("./").unwrap_or(s).to_string()
}
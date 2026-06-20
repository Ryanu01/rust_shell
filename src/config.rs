use std::collections::HashMap;
use std::fs;
use std::sync::{LazyLock, Mutex};

pub(crate) static CONFIG: LazyLock<Mutex<RushConfig>> =
    LazyLock::new(|| Mutex::new(RushConfig::default()));

pub(crate) struct RushConfig {
    pub prompt_color: String,
    pub default_array_size: usize,
    raw: HashMap<String, String>,
}

impl RushConfig {
    fn default() -> Self {
        Self {
            prompt_color: "green".to_string(),
            default_array_size: 8,
            raw: HashMap::new(),
        }
    }

    #[allow(dead_code)]
    pub fn get(&self, key: &str) -> Option<&String> {
        self.raw.get(key)
    }
}

pub(crate) fn load_config() {
    let path = dirs_or_default();
    let contents = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return,
    };

    let mut config = RushConfig::default();
    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim().to_string();
            let value = value.trim().to_string();
            config.raw.insert(key.clone(), value.clone());
            match key.as_str() {
                "prompt_color" => config.prompt_color = value,
                "default_array_size" => {
                    if let Ok(n) = value.parse() {
                        config.default_array_size = n;
                    }
                }
                _ => {}
            }
        }
    }

    let mut store = CONFIG.lock().unwrap();
    *store = config;
}

fn dirs_or_default() -> String {
    if let Ok(home) = std::env::var("HOME") {
        format!("{}/.rushrc", home)
    } else {
        ".rushrc".to_string()
    }
}

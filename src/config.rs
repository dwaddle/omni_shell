use std::collections::HashMap;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;
use shellexpand;

#[derive(Deserialize, Default, Debug)]
pub struct OmniConfig {
    #[serde(default)]
    pub aliases: HashMap<String, String>,
}

pub fn get_config_path() -> PathBuf {
    let mut path = PathBuf::from(shellexpand::tilde("~/.config/omni").into_owned());
    fs::create_dir_all(&path).unwrap_or(());
    path.push("omni.toml");
    path
}

pub fn get_history_path() -> PathBuf {
    let mut path = PathBuf::from(shellexpand::tilde("~/.config/omni").into_owned());
    fs::create_dir_all(&path).unwrap_or(());
    path.push("history.txt");
    path
}

fn load_external_aliases(aliases: &mut HashMap<String, String>) {
    let alias_path = PathBuf::from(shellexpand::tilde("~/.alias/alias").into_owned());
    if let Ok(content) = fs::read_to_string(alias_path) {
        for line in content.lines() {
            let trimmed = line.trim();
            // Zoek naar regels die beginnen met 'alias '
            if trimmed.starts_with("alias ") {
                // Split op het eerste '=' teken
                let parts: Vec<&str> = trimmed[6..].splitn(2, '=').collect();
                if parts.len() == 2 {
                    let key = parts[0].trim().to_string();
                    // Strip eventuele enkele of dubbele quotes van de waarde
                    let value = parts[1].trim()
                        .trim_matches('\'')
                        .trim_matches('"')
                        .to_string();
                    
                    aliases.insert(key, value);
                }
            }
        }
    }
}

pub fn load_config() -> OmniConfig {
    let config_path = get_config_path();
    if !config_path.exists() {
        let default_toml = "[aliases]\nls = \"eza --icons=always --color=always\"\nll = \"eza -la --icons=always --color=always\"\nupdate = \"sudo pacman -Syu\"\n";
        let _ = fs::write(&config_path, default_toml);
    }
    let mut config = match fs::read_to_string(&config_path) {
        Ok(content) => toml::from_str(&content).unwrap_or_else(|_| OmniConfig::default()),
        Err(_) => OmniConfig::default(),
    };

    // Voeg externe bash-style aliases toe (overschrijft toml als dezelfde key bestaat)
    load_external_aliases(&mut config.aliases);

    config
}

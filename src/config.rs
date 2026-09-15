use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct G13Config {
    #[serde(default = "default_profile_name")]
    pub profile_name: String,
    #[serde(default = "default_color")]
    pub color: [u8; 3],
    #[serde(default)]
    pub keys: HashMap<String, String>,
    #[serde(default)]
    pub profiles: HashMap<String, serde_json::Value>,
}

fn default_profile_name() -> String {
    "default".to_string()
}

fn default_color() -> [u8; 3] {
    [255, 0, 0]
}

impl Default for G13Config {
    fn default() -> Self {
        Self {
            profile_name: default_profile_name(),
            color: default_color(),
            keys: HashMap::new(),
            profiles: HashMap::new(),
        }
    }
}

#[allow(dead_code)]
pub fn get_profiles_dir() -> PathBuf {
    let dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from(".")).join("g13_nexus").join("profiles");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

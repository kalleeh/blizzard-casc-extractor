use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct SpriteMapping {
    #[serde(flatten)]
    pub entries: HashMap<String, String>,
}

impl SpriteMapping {
    pub fn load(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        Ok(Self::parse_str(&content))
    }

    /// Parse the mapping from an in-memory string.
    ///
    /// Exists so callers can use the YAML compiled into the binary rather than
    /// reading it from a relative path at runtime.
    pub fn parse_str(content: &str) -> Self {
        // Simple YAML parser for our format
        let mut entries = HashMap::new();
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if let Some((key, value)) = line.split_once(':') {
                let key = key.trim().to_string();
                let value = value.trim().to_string();
                entries.insert(key, value);
            }
        }

        Self { entries }
    }

    pub fn get_category_entries(&self, category: &str) -> Vec<(String, String)> {
        self.entries
            .iter()
            .filter(|(k, _)| k.starts_with(category))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }
}

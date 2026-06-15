use anyhow::{Context, Result};
use directories::ProjectDirs;
use ini::Ini;
use std::collections::HashMap;
use std::path::PathBuf;

pub const SERVICE_NAME: &str = "keepassxc-unlocker";

pub struct Config {
    conf: Ini,
    path: PathBuf,
}

impl Config {
    pub fn load() -> Result<Self> {
        let path = Self::get_config_path()?;

        let conf = if path.exists() {
            Ini::load_from_file(&path)
                .map_err(|e| anyhow::anyhow!("Failed to load config: {}", e))?
        } else {
            Ini::new()
        };

        Ok(Self { conf, path })
    }

    fn get_config_path() -> Result<PathBuf> {
        if let Some(proj_dirs) = ProjectDirs::from("", "", "keepassxc-unlocker") {
            let mut path = proj_dirs.config_dir().to_path_buf();
            path.pop(); // Go up from 'keepassxc-unlocker'
            path.push("keepassxc-unlockerrc");
            Ok(path)
        } else {
            anyhow::bail!("Could not determine config directory")
        }
    }

    pub fn get_service_name(&self) -> String {
        self.conf
            .get_from(Some("monitor"), "service")
            .unwrap_or(SERVICE_NAME)
            .to_string()
    }

    pub fn get_process_name(&self) -> String {
        self.conf
            .get_from(Some("monitor"), "process")
            .unwrap_or("keepassxc")
            .to_string()
    }

    pub fn get_autounlock_interval(&self) -> u64 {
        self.conf
            .get_from(Some("monitor"), "autounlock")
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0)
    }

    pub fn get_databases(&self) -> HashMap<String, String> {
        let mut databases = HashMap::new();
        if let Some(section) = self.conf.section(Some("databases")) {
            for (key, value) in section.iter() {
                databases.insert(key.to_string(), value.to_string());
            }
        }
        databases
    }

    pub fn add_database(&mut self, database: &str) {
        self.conf
            .with_section(Some("databases"))
            .set(database, "enabled");
    }

    pub fn remove_database(&mut self, database: &str) {
        if let Some(section) = self.conf.section_mut(Some("databases")) {
            section.remove(database);
        }
    }

    pub fn save(&self) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        self.conf
            .write_to_file(&self.path)
            .context("Failed to write config file")?;
        Ok(())
    }
}

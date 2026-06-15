use anyhow::{Context, Result};
use keyring::Entry;

pub struct Keyring {
    service: String,
}

impl Keyring {
    pub fn new(service: String) -> Self {
        Self { service }
    }

    pub fn set_password(&self, target: &str, password: &str) -> Result<()> {
        // Explicitly use Secret Service to match Python's common behavior
        let entry = Entry::new(&self.service, target)
            .map_err(|e| anyhow::anyhow!("Keyring error: {}", e))?;
        entry
            .set_password(password)
            .context("Failed to set password in keyring")?;
        Ok(())
    }

    pub fn get_password(&self, target: &str) -> Result<String> {
        let entry = Entry::new(&self.service, target)
            .map_err(|e| anyhow::anyhow!("Keyring error: {}", e))?;
        entry
            .get_password()
            .context("Failed to get password from keyring")
    }

    pub fn delete_password(&self, target: &str) -> Result<()> {
        let entry = Entry::new(&self.service, target)
            .map_err(|e| anyhow::anyhow!("Keyring error: {}", e))?;
        // delete_credential is more robust for some backends
        entry
            .delete_credential()
            .context("Failed to delete password from keyring")
    }
}

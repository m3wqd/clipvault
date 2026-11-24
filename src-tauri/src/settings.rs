use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub max_items: usize,
    pub max_bytes: usize,
    pub retention_days: u32,
    pub incognito: bool,
    pub close_to_tray: bool,
    pub theme: String,
    pub language: String,
    pub shortcut: String,
    pub skip_sensitive: bool,
    pub excluded_apps: Vec<String>,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            max_items: 1000,
            max_bytes: 1024 * 1024,
            retention_days: 30,
            incognito: false,
            close_to_tray: true,
            theme: "dark".into(),
            language: "en".into(),
            shortcut: "Ctrl+Shift+V".into(),
            skip_sensitive: false,
            excluded_apps: Vec::new(),
        }
    }
}
impl Settings {
    pub fn validate(&self) -> crate::error::Result<()> {
        if self.excluded_apps.len() > 100
            || self
                .excluded_apps
                .iter()
                .any(|s| s.is_empty() || s.len() > 200)
            || !(10..=100000).contains(&self.max_items)
            || !(1024..=16 * 1024 * 1024).contains(&self.max_bytes)
            || self.retention_days > 3650
            || !["dark", "light"].contains(&self.theme.as_str())
            || !["en", "ru"].contains(&self.language.as_str())
            || self.shortcut.len() > 100
        {
            return Err(crate::error::Error::Invalid(
                "Settings are outside allowed limits".into(),
            ));
        }
        Ok(())
    }
}
pub fn potentially_sensitive(text: &str) -> bool {
    let t = text.to_ascii_lowercase();
    [
        "-----begin private key",
        "-----begin rsa private key",
        "authorization: bearer ",
        "ghp_",
        "github_pat_",
        "sk_live_",
    ]
    .iter()
    .any(|prefix| t.contains(prefix))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validates_and_detects_limited_patterns() {
        assert!(Settings::default().validate().is_ok());
        assert!(Settings {
            max_bytes: usize::MAX,
            ..Default::default()
        }
        .validate()
        .is_err());
        assert!(potentially_sensitive("Authorization: Bearer example"));
        assert!(!potentially_sensitive("Hello there"));
    }
}

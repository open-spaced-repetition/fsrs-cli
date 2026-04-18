use anyhow::{Context, Result, bail};
use fsrs::{DEFAULT_PARAMETERS, FSRS};
use serde::{Deserialize, Serialize};
use std::{
    env, fs,
    path::{Path, PathBuf},
};

const CONFIG_DIR_ENV: &str = "FSRS_CLI_CONFIG_DIR";
const CONFIG_FILE_NAME: &str = "config.json";
pub const DEFAULT_RETENTION: f32 = 0.9;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConfigKey {
    Parameters,
    Retention,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ConfigValue {
    Parameters(Vec<f32>),
    Retention(f32),
}

impl ConfigValue {
    pub fn into_parameters(self) -> Option<Vec<f32>> {
        match self {
            Self::Parameters(values) => Some(values),
            Self::Retention(_) => None,
        }
    }

    pub fn into_retention(self) -> Option<f32> {
        match self {
            Self::Parameters(_) => None,
            Self::Retention(value) => Some(value),
        }
    }
}

pub struct ConfigStore {
    path: PathBuf,
    config: CliConfig,
}

impl ConfigStore {
    pub fn load() -> Result<Self> {
        let path = config_file_path()?;
        let config = load_config(&path)?;
        Ok(Self { path, config })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn get(&self, key: ConfigKey) -> Result<Option<ConfigValue>> {
        let value = match key {
            ConfigKey::Parameters => self
                .config
                .defaults
                .parameters
                .clone()
                .map(ConfigValue::Parameters),
            ConfigKey::Retention => self.config.defaults.retention.map(ConfigValue::Retention),
        };

        if let Some(ref value) = value {
            validate_value(key, value)?;
        }

        Ok(value)
    }

    pub fn has(&self, key: ConfigKey) -> Result<bool> {
        Ok(self.get(key)?.is_some())
    }

    pub fn set(&mut self, key: ConfigKey, value: ConfigValue) -> Result<()> {
        validate_value(key, &value)?;

        match (key, value) {
            (ConfigKey::Parameters, ConfigValue::Parameters(values)) => {
                self.config.defaults.parameters = Some(values);
            }
            (ConfigKey::Retention, ConfigValue::Retention(value)) => {
                self.config.defaults.retention = Some(value);
            }
            _ => bail!("config key does not match config value"),
        }

        Ok(())
    }

    pub fn reset(&mut self, key: ConfigKey) {
        match key {
            ConfigKey::Parameters => self.config.defaults.parameters = None,
            ConfigKey::Retention => self.config.defaults.retention = None,
        }
    }

    pub fn save(&self) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!(
                    "failed to create CLI config directory: {}",
                    parent.display()
                )
            })?;
        }

        save_config(&self.config, &self.path)
    }

    pub fn saved_parameters(&self) -> Result<Option<Vec<f32>>> {
        Ok(self
            .get(ConfigKey::Parameters)?
            .and_then(ConfigValue::into_parameters))
    }

    pub fn active_parameters(&self) -> Result<Vec<f32>> {
        Ok(self
            .saved_parameters()?
            .unwrap_or_else(|| DEFAULT_PARAMETERS.to_vec()))
    }

    pub fn saved_retention(&self) -> Result<Option<f32>> {
        Ok(self
            .get(ConfigKey::Retention)?
            .and_then(ConfigValue::into_retention))
    }

    pub fn active_retention(&self) -> Result<f32> {
        Ok(self.saved_retention()?.unwrap_or(DEFAULT_RETENTION))
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct CliConfig {
    #[serde(default)]
    defaults: ConfigDefaults,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct ConfigDefaults {
    parameters: Option<Vec<f32>>,
    retention: Option<f32>,
}

pub fn resolve_parameters(explicit: Option<Vec<f32>>) -> Result<Vec<f32>> {
    Ok(match explicit {
        Some(parameters) => parameters,
        None => ConfigStore::load()?.active_parameters()?,
    })
}

pub fn resolve_retention(explicit: Option<f32>) -> Result<f32> {
    let value = match explicit {
        Some(retention) => retention,
        None => ConfigStore::load()?.active_retention()?,
    };
    validate_value(ConfigKey::Retention, &ConfigValue::Retention(value))?;
    Ok(value)
}

pub fn format_display_path(path: &Path) -> String {
    let escaped = path.display().to_string().replace('"', "\\\"");
    format!("\"{escaped}\"")
}

pub fn format_display_path_with_status(path: &Path) -> String {
    format!(
        "{} ({})",
        format_display_path(path),
        if path.exists() {
            "created"
        } else {
            "not created"
        }
    )
}

pub fn config_file_path() -> Result<PathBuf> {
    let config_dir = config_dir()?;
    Ok(config_dir.join(CONFIG_FILE_NAME))
}

pub fn format_source_label(path: &Path, has_saved_value: bool) -> String {
    if has_saved_value {
        format!(
            "saved custom value ({})",
            format_display_path_with_status(path)
        )
    } else {
        "built-in default".to_string()
    }
}

fn load_config(path: &Path) -> Result<CliConfig> {
    if !path.exists() {
        return Ok(CliConfig::default());
    }

    let data = fs::read(path)
        .with_context(|| format!("failed to read CLI config file: {}", path.display()))?;
    serde_json::from_slice(&data)
        .with_context(|| format!("failed to parse CLI config file: {}", path.display()))
}

fn save_config(config: &CliConfig, path: &Path) -> Result<()> {
    if is_empty_config(config) {
        if path.exists() {
            fs::remove_file(path)
                .with_context(|| format!("failed to remove CLI config file: {}", path.display()))?;
        }
        return Ok(());
    }

    let data = serde_json::to_vec_pretty(config)?;
    fs::write(path, data)
        .with_context(|| format!("failed to write CLI config file: {}", path.display()))?;
    Ok(())
}

fn is_empty_config(config: &CliConfig) -> bool {
    config.defaults.parameters.is_none() && config.defaults.retention.is_none()
}

fn validate_value(key: ConfigKey, value: &ConfigValue) -> Result<()> {
    match (key, value) {
        (ConfigKey::Parameters, ConfigValue::Parameters(values)) => validate_parameters(values),
        (ConfigKey::Retention, ConfigValue::Retention(value)) => validate_retention(*value),
        _ => bail!("config key does not match config value"),
    }
}

fn validate_parameters(parameters: &[f32]) -> Result<()> {
    if parameters.len() != DEFAULT_PARAMETERS.len() {
        bail!(
            "expected {} FSRS parameters, got {}",
            DEFAULT_PARAMETERS.len(),
            parameters.len()
        );
    }
    if parameters.iter().any(|value| !value.is_finite()) {
        bail!("FSRS parameters must be finite numbers");
    }
    FSRS::new(parameters).context("invalid FSRS parameters")?;
    Ok(())
}

fn validate_retention(retention: f32) -> Result<()> {
    if !retention.is_finite() {
        bail!("retention must be a finite number");
    }
    if !(0.0..1.0).contains(&retention) {
        bail!("retention must be between 0 and 1");
    }
    Ok(())
}

fn config_dir() -> Result<PathBuf> {
    if let Some(path) = env::var_os(CONFIG_DIR_ENV) {
        return Ok(PathBuf::from(path));
    }

    #[cfg(target_os = "windows")]
    {
        let appdata = env_path("APPDATA")?;
        Ok(appdata.join("fsrs-cli"))
    }

    #[cfg(target_os = "macos")]
    {
        let home = env_path("HOME")?;
        Ok(home.join("Library/Application Support/fsrs-cli"))
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        if let Some(path) = env::var_os("XDG_CONFIG_HOME") {
            return Ok(PathBuf::from(path).join("fsrs-cli"));
        }

        let home = env_path("HOME")?;
        Ok(home.join(".config/fsrs-cli"))
    }
}

fn env_path(key: &str) -> Result<PathBuf> {
    env::var_os(key)
        .map(PathBuf::from)
        .ok_or_else(|| anyhow::anyhow!("environment variable {key} is not set"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn store_at(tempdir: &TempDir) -> ConfigStore {
        ConfigStore {
            path: tempdir.path().join(CONFIG_FILE_NAME),
            config: CliConfig::default(),
        }
    }

    #[test]
    fn test_store_set_get_and_reload() -> Result<()> {
        let tempdir = TempDir::new()?;
        let mut store = store_at(&tempdir);
        let parameters = vec![
            0.5, 1.0, 2.0, 6.0, 5.0, 0.8, 3.0, 0.001, 1.8, 0.2, 0.8, 1.5, 0.06, 0.26, 1.6, 0.6,
            1.9, 0.5, 0.09, 0.07, 0.15,
        ];

        store.set(
            ConfigKey::Parameters,
            ConfigValue::Parameters(parameters.clone()),
        )?;
        store.set(ConfigKey::Retention, ConfigValue::Retention(0.9))?;
        store.save()?;

        let reloaded = ConfigStore {
            path: store.path.clone(),
            config: load_config(&store.path)?,
        };

        assert_eq!(
            reloaded.get(ConfigKey::Parameters)?,
            Some(ConfigValue::Parameters(parameters))
        );
        assert_eq!(
            reloaded.get(ConfigKey::Retention)?,
            Some(ConfigValue::Retention(0.9))
        );
        Ok(())
    }

    #[test]
    fn test_store_rejects_invalid_retention() {
        let tempdir = TempDir::new().unwrap();
        let mut store = store_at(&tempdir);

        let error = store
            .set(ConfigKey::Retention, ConfigValue::Retention(1.2))
            .unwrap_err();

        assert!(
            error
                .to_string()
                .contains("retention must be between 0 and 1")
        );
    }
}

use std::path::Path;

use anyhow::Result;
use serde::Deserialize;

const CONFIG_FILE: &str = "jbundle.toml";

#[derive(Debug, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct ProjectConfig {
    pub java_version: Option<u8>,
    pub target: Option<String>,
    pub shrink: Option<bool>,
    pub jvm_args: Option<Vec<String>>,
    pub profile: Option<String>,
    pub appcds: Option<bool>,
    pub crac: Option<bool>,
}

pub fn load_project_config(dir: &Path) -> Result<Option<ProjectConfig>> {
    let config_path = dir.join(CONFIG_FILE);
    if !config_path.exists() {
        return Ok(None);
    }

    tracing::info!("loading config from {}", config_path.display());
    let content = std::fs::read_to_string(&config_path)?;
    let config: ProjectConfig =
        toml::from_str(&content).map_err(|e| anyhow::anyhow!("invalid {}: {}", CONFIG_FILE, e))?;
    Ok(Some(config))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn parse_full_config() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join(CONFIG_FILE),
            r#"
java_version = 17
target = "linux-x64"
shrink = true
jvm_args = ["-Xmx512m", "-XX:+UseZGC"]
profile = "cli"
appcds = false
crac = true
"#,
        )
        .unwrap();

        let config = load_project_config(dir.path()).unwrap().unwrap();
        assert_eq!(config.java_version, Some(17));
        assert_eq!(config.target.as_deref(), Some("linux-x64"));
        assert_eq!(config.shrink, Some(true));
        assert_eq!(
            config.jvm_args,
            Some(vec!["-Xmx512m".to_string(), "-XX:+UseZGC".to_string()])
        );
        assert_eq!(config.profile.as_deref(), Some("cli"));
        assert_eq!(config.appcds, Some(false));
        assert_eq!(config.crac, Some(true));
    }

    #[test]
    fn parse_partial_config() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join(CONFIG_FILE), "java_version = 21\n").unwrap();

        let config = load_project_config(dir.path()).unwrap().unwrap();
        assert_eq!(config.java_version, Some(21));
        assert_eq!(config.target, None);
        assert_eq!(config.shrink, None);
        assert_eq!(config.jvm_args, None);
    }

    #[test]
    fn parse_empty_config() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join(CONFIG_FILE), "").unwrap();

        let config = load_project_config(dir.path()).unwrap().unwrap();
        assert_eq!(config.java_version, None);
        assert_eq!(config.shrink, None);
    }

    #[test]
    fn missing_file_returns_none() {
        let dir = tempdir().unwrap();
        let result = load_project_config(dir.path()).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn invalid_toml_returns_error() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join(CONFIG_FILE), "not valid [[[toml").unwrap();

        let result = load_project_config(dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn unknown_field_returns_error() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join(CONFIG_FILE), "unknown_field = true\n").unwrap();

        let result = load_project_config(dir.path());
        assert!(result.is_err());
    }
}

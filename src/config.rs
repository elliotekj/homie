use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::strategy::Strategy;

/// Global config at ~/.config/homie/config.toml
/// Optional - only for shared settings, vars, and env passthrough
#[derive(Debug, Clone, Deserialize, Default)]
pub struct GlobalConfig {
    #[serde(default)]
    pub settings: Settings,
    #[serde(default)]
    pub vars: HashMap<String, String>,
    #[serde(default)]
    pub env: EnvConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Settings {
    #[serde(default = "default_backup_suffix")]
    pub backup_suffix: String,
    #[serde(default)]
    pub replaceable_paths: Vec<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            backup_suffix: default_backup_suffix(),
            replaceable_paths: Vec::new(),
        }
    }
}

fn default_backup_suffix() -> String {
    ".backup.%Y%m%d%H%M%S".to_string()
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct EnvConfig {
    #[serde(default)]
    pub pass_through: Vec<String>,
}

/// Per-repo config at <repo>/homie.toml
#[derive(Debug, Deserialize)]
pub struct RepoConfig {
    /// Required: target directory for symlinks (e.g., "~")
    pub target: String,
    /// Optional: repo-specific variables
    #[serde(default)]
    pub vars: HashMap<String, String>,
    #[serde(default)]
    pub defaults: RepoDefaults,
    #[serde(default)]
    pub strategies: HashMap<String, Strategy>,
    #[serde(default)]
    pub ignore: IgnoreConfig,
    /// Optional: external imports (local paths or git repos)
    #[serde(default)]
    pub imports: Vec<ImportConfig>,
}

#[derive(Debug, Deserialize)]
pub struct RepoDefaults {
    #[serde(default)]
    pub strategy: Strategy,
}

impl Default for RepoDefaults {
    fn default() -> Self {
        Self {
            strategy: Strategy::File,
        }
    }
}

#[derive(Debug, Deserialize, Default)]
pub struct IgnoreConfig {
    #[serde(default)]
    pub paths: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct PathRemap {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ImportConfig {
    pub name: Option<String>,
    pub source: String,
    #[serde(rename = "ref")]
    pub git_ref: Option<String>,
    #[serde(default = "default_import_paths")]
    pub paths: Vec<String>,
    #[serde(default)]
    pub remap: Vec<PathRemap>,
}

fn default_import_paths() -> Vec<String> {
    vec!["*".to_string()]
}

/// Default paths to always ignore in every repo
const DEFAULT_IGNORES: &[&str] = &[
    "homie.toml",
    ".git",
    ".git/**",
    ".homie",
    ".homie/**",
    ".DS_Store",
    "**/.DS_Store",
    "README.md",
    "README",
    "LICENSE",
    "LICENSE.md",
    ".gitignore",
];

impl GlobalConfig {
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;

        if !config_path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(&config_path)
            .with_context(|| format!("Failed to read config file: {}", config_path.display()))?;

        toml::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {}", config_path.display()))
    }

    pub fn config_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .context("Could not determine config directory")?
            .join("homie");

        Ok(config_dir.join("config.toml"))
    }

    pub fn expanded_replaceable_paths(&self) -> Vec<PathBuf> {
        self.settings
            .replaceable_paths
            .iter()
            .map(|p| PathBuf::from(shellexpand::tilde(p).as_ref()))
            .collect()
    }
}

/// Returns ~/.homie/repos
pub fn repos_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().context("Could not determine home directory")?;
    Ok(home.join(".homie/repos"))
}

impl RepoConfig {
    pub fn load(repo_path: &Path) -> Result<Self> {
        let config_path = repo_path.join("homie.toml");

        if !config_path.exists() {
            bail!(
                "No homie.toml found in {}. Run 'homie init' to create one.",
                repo_path.display()
            );
        }

        let content = fs::read_to_string(&config_path)
            .with_context(|| format!("Failed to read repo config: {}", config_path.display()))?;

        toml::from_str(&content)
            .with_context(|| format!("Failed to parse repo config: {}", config_path.display()))
    }

    pub fn expanded_target(&self) -> PathBuf {
        let expanded = shellexpand::tilde(&self.target);
        PathBuf::from(expanded.as_ref())
    }

    pub fn strategy_for_path(&self, path: &str) -> Strategy {
        for (pattern, strategy) in &self.strategies {
            if path.starts_with(pattern) || glob_match(pattern, path) {
                return *strategy;
            }
        }
        self.defaults.strategy
    }

    pub fn is_ignored(&self, path: &str) -> bool {
        // Check default ignores first
        for pattern in DEFAULT_IGNORES {
            if glob_match(pattern, path) {
                return true;
            }
        }

        // Check user-defined ignores
        for pattern in &self.ignore.paths {
            if glob_match(pattern, path) {
                return true;
            }
        }

        false
    }
}

fn glob_match(pattern: &str, path: &str) -> bool {
    if let Ok(glob_pattern) = glob::Pattern::new(pattern) {
        glob_pattern.matches(path)
    } else {
        pattern == path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_global_config() {
        let config = GlobalConfig::default();
        assert!(config.vars.is_empty());
        assert_eq!(config.settings.backup_suffix, ".backup.%Y%m%d%H%M%S");
    }

    #[test]
    fn test_parse_global_config() {
        let toml = r#"
[settings]
backup_suffix = ".bak"
replaceable_paths = ["~/dev/project"]

[vars]
email = "test@example.com"

[env]
pass_through = ["API_KEY"]
"#;

        let config: GlobalConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.settings.backup_suffix, ".bak");
        assert_eq!(config.vars.get("email").unwrap(), "test@example.com");
        assert_eq!(config.env.pass_through, vec!["API_KEY"]);
    }

    #[test]
    fn test_parse_repo_config() {
        let toml = r#"
target = "~"

[vars]
git_user = "testuser"

[defaults]
strategy = "file"

[strategies]
".config/nvim" = "directory"
".local/bin" = "contents"

[ignore]
paths = ["*.swp"]
"#;

        let config: RepoConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.target, "~");
        assert_eq!(config.vars.get("git_user").unwrap(), "testuser");
        assert_eq!(config.defaults.strategy, Strategy::File);
        assert_eq!(
            config.strategies.get(".config/nvim"),
            Some(&Strategy::Directory)
        );
        assert!(config.is_ignored("test.swp"));
        assert!(config.is_ignored(".git"));
        assert!(config.is_ignored("homie.toml"));
    }

    #[test]
    fn test_default_ignores() {
        let toml = r#"target = "~""#;
        let config: RepoConfig = toml::from_str(toml).unwrap();

        assert!(config.is_ignored("homie.toml"));
        assert!(config.is_ignored(".git"));
        assert!(config.is_ignored("README.md"));
        assert!(config.is_ignored("LICENSE"));
        assert!(!config.is_ignored(".zshrc"));
    }

    #[test]
    fn test_parse_import_with_remap() {
        let toml = r#"
target = "~"

[[imports]]
source = "https://github.com/user/repo.git"
name = "myimport"
paths = ["commands/**"]

[[imports.remap]]
from = "commands"
to = ".claude/commands"
"#;

        let config: RepoConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.imports.len(), 1);
        assert_eq!(config.imports[0].name, Some("myimport".to_string()));
        assert_eq!(config.imports[0].remap.len(), 1);
        assert_eq!(config.imports[0].remap[0].from, "commands");
        assert_eq!(config.imports[0].remap[0].to, ".claude/commands");
    }

    #[test]
    fn test_parse_import_with_inline_remap() {
        let toml = r#"
target = "~"

[[imports]]
source = "https://github.com/user/repo.git"
paths = ["commands/**"]
remap = [{ from = "commands", to = ".claude/commands" }]
"#;

        let config: RepoConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.imports.len(), 1);
        assert_eq!(config.imports[0].remap.len(), 1);
        assert_eq!(config.imports[0].remap[0].from, "commands");
        assert_eq!(config.imports[0].remap[0].to, ".claude/commands");
    }
}

use anyhow::{bail, Context, Result};
use colored::Colorize;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::config::{ImportConfig, PathRemap};

#[derive(Debug)]
pub struct Import {
    pub name: String,
    pub source: ImportSource,
    pub local_path: PathBuf,
    pub paths: Vec<String>,
    pub remap: Vec<PathRemap>,
}

#[derive(Debug)]
pub enum ImportSource {
    Local(PathBuf),
    Git { url: String, git_ref: Option<String> },
}

impl Import {
    pub fn ensure_available(&self, repo_path: &Path, dry_run: bool) -> Result<()> {
        match &self.source {
            ImportSource::Local(path) => {
                if !path.exists() {
                    bail!("Import source does not exist: {}", path.display());
                }
                Ok(())
            }
            ImportSource::Git { url, git_ref } => {
                let import_dir = repo_path.join(".homie/imports").join(&self.name);

                if import_dir.exists() {
                    // Pull latest
                    println!("  {} {} (pulling)", "↓".cyan(), self.name);
                    if !dry_run {
                        git_pull(&import_dir, git_ref.as_deref())?;
                    }
                } else {
                    // Clone
                    println!("  {} {} (cloning)", "↓".cyan(), self.name);
                    if !dry_run {
                        git_clone(url, &import_dir, git_ref.as_deref())?;
                    }
                }
                Ok(())
            }
        }
    }

    pub fn source_path(&self) -> &Path {
        &self.local_path
    }

    pub fn includes_path(&self, relative_path: &str) -> bool {
        self.paths.iter().any(|pattern| {
            pattern == "*"
                || relative_path == pattern
                || relative_path.starts_with(&format!("{}/", pattern))
                || glob::Pattern::new(pattern)
                    .map(|p| p.matches(relative_path))
                    .unwrap_or(false)
        })
    }

    pub fn remap_path(&self, relative_path: &Path) -> PathBuf {
        self.remap
            .iter()
            .find_map(|r| {
                relative_path
                    .strip_prefix(&r.from)
                    .ok()
                    .map(|stripped| PathBuf::from(&r.to).join(stripped))
            })
            .unwrap_or_else(|| relative_path.to_path_buf())
    }
}

pub fn resolve_import(config: &ImportConfig, repo_path: &Path) -> Result<Import> {
    let source = &config.source;

    if is_git_url(source) {
        let name = config
            .name
            .clone()
            .unwrap_or_else(|| derive_name_from_url(source));

        let local_path = repo_path.join(".homie/imports").join(&name);

        Ok(Import {
            name,
            source: ImportSource::Git {
                url: source.clone(),
                git_ref: config.git_ref.clone(),
            },
            local_path,
            paths: config.paths.clone(),
            remap: config.remap.clone(),
        })
    } else {
        // Local path
        let expanded = shellexpand::tilde(source);
        let path = PathBuf::from(expanded.as_ref());

        let name = config.name.clone().unwrap_or_else(|| {
            path.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "import".to_string())
        });

        Ok(Import {
            name,
            source: ImportSource::Local(path.clone()),
            local_path: path,
            paths: config.paths.clone(),
            remap: config.remap.clone(),
        })
    }
}

fn is_git_url(source: &str) -> bool {
    source.starts_with("git@")
        || source.starts_with("https://github.com")
        || source.starts_with("https://gitlab.com")
        || source.starts_with("https://bitbucket.org")
        || source.ends_with(".git")
        || source.contains("://") && source.contains("git")
}

fn derive_name_from_url(url: &str) -> String {
    let url = url.trim_end_matches('/').trim_end_matches(".git");
    url.rsplit('/')
        .next()
        .or_else(|| url.rsplit(':').next())
        .unwrap_or("import")
        .to_string()
}

fn git_clone(url: &str, dest: &Path, git_ref: Option<&str>) -> Result<()> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
    }

    let dest_str = dest.to_string_lossy();
    let mut args = vec!["clone", "--depth", "1"];
    if let Some(ref_name) = git_ref {
        args.push("--branch");
        args.push(ref_name);
    }
    args.push(url);
    args.push(&dest_str);

    let output = Command::new("git")
        .args(&args)
        .output()
        .context("Failed to execute git clone")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git clone failed: {}", stderr.trim());
    }

    Ok(())
}

fn git_pull(repo_path: &Path, git_ref: Option<&str>) -> Result<()> {
    // If a ref is specified, fetch and checkout that ref
    if let Some(ref_name) = git_ref {
        let output = Command::new("git")
            .args(["fetch", "origin", ref_name])
            .current_dir(repo_path)
            .output()
            .context("Failed to execute git fetch")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("git fetch failed: {}", stderr.trim());
        }

        let output = Command::new("git")
            .args(["checkout", ref_name])
            .current_dir(repo_path)
            .output()
            .context("Failed to execute git checkout")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("git checkout failed: {}", stderr.trim());
        }
    } else {
        // Just pull
        let output = Command::new("git")
            .args(["pull", "--ff-only"])
            .current_dir(repo_path)
            .output()
            .context("Failed to execute git pull")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("git pull failed: {}", stderr.trim());
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_git_url() {
        assert!(is_git_url("git@github.com:user/repo.git"));
        assert!(is_git_url("https://github.com/user/repo.git"));
        assert!(is_git_url("https://github.com/user/repo"));
        assert!(is_git_url("https://gitlab.com/user/repo"));
        assert!(!is_git_url("~/dotfiles"));
        assert!(!is_git_url("/home/user/dotfiles"));
    }

    #[test]
    fn test_derive_name_from_url() {
        assert_eq!(
            derive_name_from_url("git@github.com:user/dotfiles.git"),
            "dotfiles"
        );
        assert_eq!(
            derive_name_from_url("https://github.com/user/my-configs.git"),
            "my-configs"
        );
        assert_eq!(
            derive_name_from_url("https://github.com/user/repo"),
            "repo"
        );
    }

    #[test]
    fn test_includes_path_wildcard() {
        let import = Import {
            name: "test".to_string(),
            source: ImportSource::Local(PathBuf::from("/tmp")),
            local_path: PathBuf::from("/tmp"),
            paths: vec!["*".to_string()],
            remap: vec![],
        };
        assert!(import.includes_path(".zshrc"));
        assert!(import.includes_path(".config/nvim/init.lua"));
    }

    #[test]
    fn test_includes_path_exact() {
        let import = Import {
            name: "test".to_string(),
            source: ImportSource::Local(PathBuf::from("/tmp")),
            local_path: PathBuf::from("/tmp"),
            paths: vec![".zshrc".to_string(), ".config/nvim".to_string()],
            remap: vec![],
        };
        assert!(import.includes_path(".zshrc"));
        assert!(import.includes_path(".config/nvim"));
        assert!(import.includes_path(".config/nvim/init.lua"));
        assert!(!import.includes_path(".bashrc"));
        assert!(!import.includes_path(".config/git/config"));
    }

    #[test]
    fn test_includes_path_glob() {
        let import = Import {
            name: "test".to_string(),
            source: ImportSource::Local(PathBuf::from("/tmp")),
            local_path: PathBuf::from("/tmp"),
            paths: vec![".config/*".to_string()],
            remap: vec![],
        };
        assert!(import.includes_path(".config/nvim"));
        assert!(import.includes_path(".config/git"));
        assert!(!import.includes_path(".zshrc"));
    }

    #[test]
    fn test_remap_path() {
        use crate::config::PathRemap;

        let import = Import {
            name: "test".to_string(),
            source: ImportSource::Local(PathBuf::from("/tmp")),
            local_path: PathBuf::from("/tmp"),
            paths: vec!["*".to_string()],
            remap: vec![
                PathRemap {
                    from: "commands".to_string(),
                    to: ".claude/commands".to_string(),
                },
            ],
        };

        // Should remap matching paths
        assert_eq!(
            import.remap_path(Path::new("commands/foo.md")),
            PathBuf::from(".claude/commands/foo.md")
        );
        assert_eq!(
            import.remap_path(Path::new("commands/sub/bar.md")),
            PathBuf::from(".claude/commands/sub/bar.md")
        );

        // Should not remap non-matching paths
        assert_eq!(
            import.remap_path(Path::new("hooks/test.sh")),
            PathBuf::from("hooks/test.sh")
        );
    }
}

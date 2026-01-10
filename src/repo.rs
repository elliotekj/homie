use anyhow::{bail, Context, Result};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::config::{repos_dir, RepoConfig};
use crate::import::{resolve_import, Import};
use crate::strategy::Strategy;

#[derive(Debug)]
pub struct Repo {
    pub name: String,
    pub path: PathBuf,
    pub target: PathBuf,
    pub config: RepoConfig,
    pub imports: Vec<Import>,
}

#[derive(Debug)]
pub struct RepoItem {
    pub source: PathBuf,
    pub target: PathBuf,
    pub relative_path: String,
    pub is_template: bool,
}

/// Discover all repos in ~/.homie/repos/
pub fn discover_repos() -> Result<Vec<Repo>> {
    let repos_path = repos_dir()?;

    if !repos_path.exists() {
        return Ok(Vec::new());
    }

    let mut repos = Vec::new();

    for entry in fs::read_dir(&repos_path)
        .with_context(|| format!("Failed to read repos directory: {}", repos_path.display()))?
    {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() && path.join("homie.toml").exists() {
            match Repo::from_path(&path) {
                Ok(repo) => repos.push(repo),
                Err(e) => {
                    eprintln!(
                        "Warning: Failed to load repo {}: {}",
                        path.display(),
                        e
                    );
                }
            }
        }
    }

    // Sort by name for consistent ordering
    repos.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(repos)
}

/// Find a specific repo by name
pub fn find_repo(name: &str) -> Result<Option<Repo>> {
    let repos_path = repos_dir()?;
    let repo_path = repos_path.join(name);

    if repo_path.is_dir() && repo_path.join("homie.toml").exists() {
        Ok(Some(Repo::from_path(&repo_path)?))
    } else {
        Ok(None)
    }
}

impl Repo {
    pub fn from_path(path: &Path) -> Result<Self> {
        if !path.exists() {
            bail!("Repo path does not exist: {}", path.display());
        }

        let config = RepoConfig::load(path)?;
        let target = config.expanded_target();

        let name = path
            .file_name()
            .context("Repo path has no name")?
            .to_string_lossy()
            .to_string();

        // Resolve imports
        let imports = config
            .imports
            .iter()
            .filter_map(|ic| match resolve_import(ic, path) {
                Ok(import) => Some(import),
                Err(e) => {
                    eprintln!("Warning: Failed to resolve import '{}': {}", ic.source, e);
                    None
                }
            })
            .collect();

        Ok(Self {
            name,
            path: path.to_path_buf(),
            target,
            config,
            imports,
        })
    }

    pub fn vars(&self) -> &HashMap<String, String> {
        &self.config.vars
    }

    /// Fetch all git imports (clone if missing, pull if exists)
    pub fn fetch_imports(&self, dry_run: bool) -> Result<()> {
        for import in &self.imports {
            import.ensure_available(&self.path, dry_run)?;
        }
        Ok(())
    }

    pub fn items(&self) -> Result<Vec<RepoItem>> {
        let mut items = Vec::new();
        let mut seen_paths: HashSet<String> = HashSet::new();

        for item in self.collect_items_from(&self.path)? {
            seen_paths.insert(item.relative_path.clone());
            items.push(item);
        }

        for import in &self.imports {
            let import_path = import.source_path();
            if !import_path.exists() {
                continue;
            }

            let Ok(import_items) = self.collect_items_from(import_path) else {
                continue;
            };

            for mut item in import_items {
                if !import.includes_path(&item.relative_path) {
                    continue;
                }

                let remapped = import.remap_path(Path::new(&item.relative_path));
                let remapped_str = remapped.to_string_lossy().to_string();

                if seen_paths.contains(&remapped_str) {
                    continue;
                }

                item.target = compute_target(&self.target, &remapped, item.is_template);
                item.relative_path = remapped_str.clone();
                seen_paths.insert(remapped_str);
                items.push(item);
            }
        }

        Ok(items)
    }

    fn collect_items_from(&self, source_root: &Path) -> Result<Vec<RepoItem>> {
        let mut items = Vec::new();
        let mut processed_dirs = HashSet::new();

        for entry in WalkDir::new(source_root)
            .min_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let source = entry.path().to_path_buf();
            let relative = source
                .strip_prefix(source_root)
                .context("Failed to strip prefix")?;
            let relative_str = relative.to_string_lossy().to_string();

            if self.config.is_ignored(&relative_str) {
                continue;
            }

            let has_processed_ancestor = relative
                .ancestors()
                .skip(1)
                .filter(|a| !a.as_os_str().is_empty())
                .any(|a| processed_dirs.contains(&a.to_path_buf()));

            if has_processed_ancestor {
                continue;
            }

            let is_template = source.extension().is_some_and(|e| e == "tmpl");

            let target = compute_target(&self.target, relative, is_template);

            if entry.file_type().is_dir() {
                if self.config.strategy_for_path(&relative_str) == Strategy::Directory {
                    processed_dirs.insert(relative.to_path_buf());
                    items.push(RepoItem {
                        source,
                        target,
                        relative_path: relative_str,
                        is_template,
                    });
                }
                continue;
            }

            items.push(RepoItem {
                source,
                target,
                relative_path: relative_str,
                is_template,
            });
        }

        Ok(items)
    }
}

fn compute_target(base: &Path, relative: &Path, is_template: bool) -> PathBuf {
    let mut target = base.join(relative);

    // Strip .tmpl extension for templates
    if is_template {
        if let Some(stem) = target.file_stem() {
            let parent = target.parent().unwrap_or(Path::new(""));
            target = parent.join(stem);
        }
    }

    target
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_target() {
        let base = PathBuf::from("/home/user");
        let relative = PathBuf::from(".config/app/config.toml");

        let target = compute_target(&base, &relative, false);
        assert_eq!(target, PathBuf::from("/home/user/.config/app/config.toml"));
    }

    #[test]
    fn test_compute_target_template() {
        let base = PathBuf::from("/home/user");
        let relative = PathBuf::from(".config/app/config.toml.tmpl");

        let target = compute_target(&base, &relative, true);
        assert_eq!(target, PathBuf::from("/home/user/.config/app/config.toml"));
    }
}

use anyhow::{Context, Result};
use chrono::Local;
use colored::Colorize;
use std::fs;
use std::os::unix::fs as unix_fs;
use std::path::{Path, PathBuf};

use crate::config::GlobalConfig;
use crate::repo::RepoItem;
use crate::template::TemplateEngine;
use crate::vars::VarResolver;

#[derive(Debug, Clone, Copy, Default)]
pub struct LinkOptions {
    pub dry_run: bool,
    pub force: bool,
    pub verbose: bool,
    pub no_fetch: bool,
}

#[derive(Debug)]
pub enum LinkResult {
    Created,
    AlreadyCorrect,
    Skipped { reason: String },
    BackedUp { backup_path: PathBuf },
    Rendered,
}

pub struct Linker {
    config: GlobalConfig,
    template_engine: TemplateEngine,
    replaceable_paths: Vec<PathBuf>,
}

impl Linker {
    pub fn new(config: GlobalConfig) -> Self {
        let replaceable_paths = config.expanded_replaceable_paths();
        Self {
            config,
            template_engine: TemplateEngine::new(),
            replaceable_paths,
        }
    }

    pub fn link_item(
        &self,
        item: &RepoItem,
        var_resolver: &VarResolver,
        repo_path: &Path,
        options: LinkOptions,
    ) -> Result<LinkResult> {
        // Handle templates differently - they render to files, not symlinks
        if item.is_template {
            return self.render_template(item, var_resolver, options);
        }

        // Check if source is a broken symlink
        if item.source.is_symlink() && !item.source.exists() {
            return Ok(LinkResult::Skipped {
                reason: "source is broken symlink".to_string(),
            });
        }

        // Check target state
        let target_state = classify_target(&item.target, repo_path, &self.replaceable_paths)?;

        match target_state {
            TargetState::NotExists => {
                self.create_symlink(&item.source, &item.target, options)
            }
            TargetState::SymlinkToRepo | TargetState::SymlinkToReplaceable => {
                // Replace with our symlink
                if !options.dry_run {
                    fs::remove_file(&item.target)
                        .with_context(|| format!("Failed to remove: {}", item.target.display()))?;
                }
                self.create_symlink(&item.source, &item.target, options)
            }
            TargetState::SymlinkToExternal(path) => {
                Ok(LinkResult::Skipped {
                    reason: format!("external symlink: {}", path.display()),
                })
            }
            TargetState::BrokenSymlink => {
                if !options.dry_run {
                    fs::remove_file(&item.target)
                        .with_context(|| format!("Failed to remove broken symlink: {}", item.target.display()))?;
                }
                self.create_symlink(&item.source, &item.target, options)
            }
            TargetState::RegularFile | TargetState::Directory => {
                if options.force {
                    let backup_path = self.backup_path(&item.target)?;
                    if !options.dry_run {
                        fs::rename(&item.target, &backup_path)
                            .with_context(|| format!("Failed to backup: {}", item.target.display()))?;
                    }
                    self.create_symlink(&item.source, &item.target, options)?;
                    Ok(LinkResult::BackedUp { backup_path })
                } else {
                    Ok(LinkResult::Skipped {
                        reason: "file exists (use --force to backup)".to_string(),
                    })
                }
            }
        }
    }

    fn create_symlink(&self, source: &Path, target: &Path, options: LinkOptions) -> Result<LinkResult> {
        if options.dry_run {
            return Ok(LinkResult::Created);
        }

        // Ensure parent directory exists
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create parent dir: {}", parent.display()))?;
        }

        // Resolve source to absolute path if it's a relative symlink
        let resolved_source = if source.is_symlink() {
            source.canonicalize()
                .with_context(|| format!("Failed to resolve symlink: {}", source.display()))?
        } else {
            source.to_path_buf()
        };

        unix_fs::symlink(&resolved_source, target)
            .with_context(|| format!("Failed to create symlink: {} -> {}", target.display(), resolved_source.display()))?;

        Ok(LinkResult::Created)
    }

    fn render_template(
        &self,
        item: &RepoItem,
        var_resolver: &VarResolver,
        options: LinkOptions,
    ) -> Result<LinkResult> {
        let vars = var_resolver.to_template_data();
        let rendered = self.template_engine.render_file(&item.source, &vars)?;

        if options.dry_run {
            return Ok(LinkResult::Rendered);
        }

        // Ensure parent directory exists
        if let Some(parent) = item.target.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create parent dir: {}", parent.display()))?;
        }

        // Check if target already exists and has same content
        if item.target.exists() {
            let existing = fs::read_to_string(&item.target).unwrap_or_default();
            if existing == rendered {
                return Ok(LinkResult::AlreadyCorrect);
            }
        }

        fs::write(&item.target, &rendered)
            .with_context(|| format!("Failed to write: {}", item.target.display()))?;

        Ok(LinkResult::Rendered)
    }

    fn backup_path(&self, path: &Path) -> Result<PathBuf> {
        let suffix = Local::now()
            .format(&self.config.settings.backup_suffix)
            .to_string();

        let file_name = path
            .file_name()
            .context("Path has no filename")?
            .to_string_lossy();

        let backup_name = format!("{}{}", file_name, suffix);
        let backup_path = path.with_file_name(backup_name);

        Ok(backup_path)
    }

    pub fn unlink_item(&self, item: &RepoItem, options: LinkOptions) -> Result<LinkResult> {
        if !item.target.exists() && !item.target.is_symlink() {
            return Ok(LinkResult::Skipped {
                reason: "does not exist".to_string(),
            });
        }

        // Only remove if it's a symlink pointing to our source (or a rendered template)
        if item.target.is_symlink() {
            let link_target = fs::read_link(&item.target)?;
            let resolved = if link_target.is_absolute() {
                link_target
            } else {
                item.target.parent().unwrap_or(Path::new("")).join(&link_target)
            };

            if resolved != item.source && !item.is_template {
                return Ok(LinkResult::Skipped {
                    reason: "symlink points elsewhere".to_string(),
                });
            }
        } else if !item.is_template {
            return Ok(LinkResult::Skipped {
                reason: "not a symlink".to_string(),
            });
        }

        if !options.dry_run {
            if item.target.is_dir() && !item.target.is_symlink() {
                fs::remove_dir_all(&item.target)?;
            } else {
                fs::remove_file(&item.target)?;
            }
        }

        Ok(LinkResult::Created) // Using Created to mean "action taken"
    }
}

#[derive(Debug)]
enum TargetState {
    NotExists,
    SymlinkToRepo,
    SymlinkToReplaceable,
    SymlinkToExternal(PathBuf),
    BrokenSymlink,
    RegularFile,
    Directory,
}

fn classify_target(target: &Path, repo_path: &Path, replaceable_paths: &[PathBuf]) -> Result<TargetState> {
    if !target.exists() && !target.is_symlink() {
        return Ok(TargetState::NotExists);
    }

    if target.is_symlink() {
        let link_target = fs::read_link(target)?;
        let resolved = if link_target.is_absolute() {
            link_target.clone()
        } else {
            target.parent().unwrap_or(Path::new("")).join(&link_target)
        };

        if !resolved.exists() {
            return Ok(TargetState::BrokenSymlink);
        }

        if resolved.starts_with(repo_path) {
            return Ok(TargetState::SymlinkToRepo);
        }

        let is_replaceable = replaceable_paths
            .iter()
            .any(|p| resolved.starts_with(p));

        return if is_replaceable {
            Ok(TargetState::SymlinkToReplaceable)
        } else {
            Ok(TargetState::SymlinkToExternal(resolved))
        };
    }

    if target.is_dir() {
        Ok(TargetState::Directory)
    } else {
        Ok(TargetState::RegularFile)
    }
}

pub fn print_result(relative_path: &str, result: &LinkResult, verbose: bool) {
    match result {
        LinkResult::Created => {
            println!("  {} {}", "✓".green(), relative_path);
        }
        LinkResult::AlreadyCorrect => {
            if verbose {
                println!("  {} {} (unchanged)", "✓".green(), relative_path.dimmed());
            }
        }
        LinkResult::Skipped { reason } => {
            println!("  {} {} ({})", "⊘".yellow(), relative_path, reason.dimmed());
        }
        LinkResult::BackedUp { backup_path } => {
            println!(
                "  {} {} (backup: {})",
                "⚠".yellow(),
                relative_path,
                backup_path.file_name().unwrap_or_default().to_string_lossy()
            );
        }
        LinkResult::Rendered => {
            println!("  {} {} (rendered)", "✓".green(), relative_path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;
    use tempfile::TempDir;

    fn create_test_linker() -> Linker {
        Linker::new(GlobalConfig::default())
    }

    fn default_options() -> LinkOptions {
        LinkOptions::default()
    }

    // Tests for classify_target

    #[test]
    fn test_classify_target_not_exists() {
        let temp = TempDir::new().unwrap();
        let target = temp.path().join("nonexistent");
        let repo_path = temp.path().join("repo");

        let state = classify_target(&target, &repo_path, &[]).unwrap();
        assert!(matches!(state, TargetState::NotExists));
    }

    #[test]
    fn test_classify_target_regular_file() {
        let temp = TempDir::new().unwrap();
        let target = temp.path().join("file.txt");
        fs::write(&target, "content").unwrap();
        let repo_path = temp.path().join("repo");

        let state = classify_target(&target, &repo_path, &[]).unwrap();
        assert!(matches!(state, TargetState::RegularFile));
    }

    #[test]
    fn test_classify_target_directory() {
        let temp = TempDir::new().unwrap();
        let target = temp.path().join("dir");
        fs::create_dir(&target).unwrap();
        let repo_path = temp.path().join("repo");

        let state = classify_target(&target, &repo_path, &[]).unwrap();
        assert!(matches!(state, TargetState::Directory));
    }

    #[test]
    fn test_classify_target_symlink_to_repo() {
        let temp = TempDir::new().unwrap();
        let repo_path = temp.path().join("repo");
        fs::create_dir(&repo_path).unwrap();
        let source = repo_path.join("file.txt");
        fs::write(&source, "content").unwrap();

        let target = temp.path().join("link");
        symlink(&source, &target).unwrap();

        let state = classify_target(&target, &repo_path, &[]).unwrap();
        assert!(matches!(state, TargetState::SymlinkToRepo));
    }

    #[test]
    fn test_classify_target_symlink_to_replaceable() {
        let temp = TempDir::new().unwrap();
        let replaceable = temp.path().join("replaceable");
        fs::create_dir(&replaceable).unwrap();
        let source = replaceable.join("file.txt");
        fs::write(&source, "content").unwrap();

        let target = temp.path().join("link");
        symlink(&source, &target).unwrap();

        let repo_path = temp.path().join("repo");
        let state = classify_target(&target, &repo_path, &[replaceable]).unwrap();
        assert!(matches!(state, TargetState::SymlinkToReplaceable));
    }

    #[test]
    fn test_classify_target_symlink_to_external() {
        let temp = TempDir::new().unwrap();
        let external = temp.path().join("external");
        fs::create_dir(&external).unwrap();
        let source = external.join("file.txt");
        fs::write(&source, "content").unwrap();

        let target = temp.path().join("link");
        symlink(&source, &target).unwrap();

        let repo_path = temp.path().join("repo");
        let state = classify_target(&target, &repo_path, &[]).unwrap();
        assert!(matches!(state, TargetState::SymlinkToExternal(_)));
    }

    #[test]
    fn test_classify_target_broken_symlink() {
        let temp = TempDir::new().unwrap();
        let target = temp.path().join("broken_link");
        symlink("/nonexistent/path", &target).unwrap();

        let repo_path = temp.path().join("repo");
        let state = classify_target(&target, &repo_path, &[]).unwrap();
        assert!(matches!(state, TargetState::BrokenSymlink));
    }

    // Tests for Linker::create_symlink

    #[test]
    fn test_create_symlink_basic() {
        let temp = TempDir::new().unwrap();
        let source = temp.path().join("source.txt");
        fs::write(&source, "content").unwrap();
        let target = temp.path().join("target.txt");

        let linker = create_test_linker();
        let result = linker.create_symlink(&source, &target, default_options()).unwrap();

        assert!(matches!(result, LinkResult::Created));
        assert!(target.is_symlink());
        assert_eq!(fs::read_to_string(&target).unwrap(), "content");
    }

    #[test]
    fn test_create_symlink_creates_parent_dirs() {
        let temp = TempDir::new().unwrap();
        let source = temp.path().join("source.txt");
        fs::write(&source, "content").unwrap();
        let target = temp.path().join("nested/deep/target.txt");

        let linker = create_test_linker();
        let result = linker.create_symlink(&source, &target, default_options()).unwrap();

        assert!(matches!(result, LinkResult::Created));
        assert!(target.is_symlink());
    }

    #[test]
    fn test_create_symlink_dry_run() {
        let temp = TempDir::new().unwrap();
        let source = temp.path().join("source.txt");
        fs::write(&source, "content").unwrap();
        let target = temp.path().join("target.txt");

        let linker = create_test_linker();
        let options = LinkOptions { dry_run: true, ..default_options() };
        let result = linker.create_symlink(&source, &target, options).unwrap();

        assert!(matches!(result, LinkResult::Created));
        assert!(!target.exists()); // Should not actually create
    }

    // Tests for Linker::link_item

    #[test]
    fn test_link_item_creates_new_symlink() {
        let temp = TempDir::new().unwrap();
        let repo_path = temp.path().join("repo");
        fs::create_dir(&repo_path).unwrap();
        let source = repo_path.join("file.txt");
        fs::write(&source, "content").unwrap();
        let target = temp.path().join("home/file.txt");

        let item = RepoItem {
            source: source.clone(),
            target: target.clone(),
            relative_path: "file.txt".to_string(),
            is_template: false,
        };

        let linker = create_test_linker();
        let var_resolver = VarResolver::new(&GlobalConfig::default(), None);
        let result = linker.link_item(&item, &var_resolver, &repo_path, default_options()).unwrap();

        assert!(matches!(result, LinkResult::Created));
        assert!(target.is_symlink());
    }

    #[test]
    fn test_link_item_skips_existing_file_without_force() {
        let temp = TempDir::new().unwrap();
        let repo_path = temp.path().join("repo");
        fs::create_dir(&repo_path).unwrap();
        let source = repo_path.join("file.txt");
        fs::write(&source, "repo content").unwrap();

        let target = temp.path().join("file.txt");
        fs::write(&target, "existing content").unwrap();

        let item = RepoItem {
            source,
            target: target.clone(),
            relative_path: "file.txt".to_string(),
            is_template: false,
        };

        let linker = create_test_linker();
        let var_resolver = VarResolver::new(&GlobalConfig::default(), None);
        let result = linker.link_item(&item, &var_resolver, &repo_path, default_options()).unwrap();

        assert!(matches!(result, LinkResult::Skipped { .. }));
        assert!(!target.is_symlink()); // Should remain a regular file
    }

    #[test]
    fn test_link_item_backs_up_existing_file_with_force() {
        let temp = TempDir::new().unwrap();
        let repo_path = temp.path().join("repo");
        fs::create_dir(&repo_path).unwrap();
        let source = repo_path.join("file.txt");
        fs::write(&source, "repo content").unwrap();

        let target = temp.path().join("file.txt");
        fs::write(&target, "existing content").unwrap();

        let item = RepoItem {
            source,
            target: target.clone(),
            relative_path: "file.txt".to_string(),
            is_template: false,
        };

        let linker = create_test_linker();
        let var_resolver = VarResolver::new(&GlobalConfig::default(), None);
        let options = LinkOptions { force: true, ..default_options() };
        let result = linker.link_item(&item, &var_resolver, &repo_path, options).unwrap();

        assert!(matches!(result, LinkResult::BackedUp { .. }));
        assert!(target.is_symlink()); // Should now be a symlink
    }

    #[test]
    fn test_link_item_replaces_broken_symlink() {
        let temp = TempDir::new().unwrap();
        let repo_path = temp.path().join("repo");
        fs::create_dir(&repo_path).unwrap();
        let source = repo_path.join("file.txt");
        fs::write(&source, "content").unwrap();

        let target = temp.path().join("file.txt");
        symlink("/nonexistent", &target).unwrap();

        let item = RepoItem {
            source: source.clone(),
            target: target.clone(),
            relative_path: "file.txt".to_string(),
            is_template: false,
        };

        let linker = create_test_linker();
        let var_resolver = VarResolver::new(&GlobalConfig::default(), None);
        let result = linker.link_item(&item, &var_resolver, &repo_path, default_options()).unwrap();

        assert!(matches!(result, LinkResult::Created));
        assert_eq!(fs::read_to_string(&target).unwrap(), "content");
    }

    #[test]
    fn test_link_item_skips_external_symlink() {
        let temp = TempDir::new().unwrap();
        let repo_path = temp.path().join("repo");
        fs::create_dir(&repo_path).unwrap();
        let source = repo_path.join("file.txt");
        fs::write(&source, "repo content").unwrap();

        let external = temp.path().join("external");
        fs::create_dir(&external).unwrap();
        let external_file = external.join("file.txt");
        fs::write(&external_file, "external content").unwrap();

        let target = temp.path().join("link.txt");
        symlink(&external_file, &target).unwrap();

        let item = RepoItem {
            source,
            target: target.clone(),
            relative_path: "link.txt".to_string(),
            is_template: false,
        };

        let linker = create_test_linker();
        let var_resolver = VarResolver::new(&GlobalConfig::default(), None);
        let result = linker.link_item(&item, &var_resolver, &repo_path, default_options()).unwrap();

        assert!(matches!(result, LinkResult::Skipped { .. }));
        // Should still point to external
        assert_eq!(fs::read_to_string(&target).unwrap(), "external content");
    }

    // Tests for Linker::unlink_item

    #[test]
    fn test_unlink_item_removes_symlink() {
        let temp = TempDir::new().unwrap();
        let source = temp.path().join("source.txt");
        fs::write(&source, "content").unwrap();
        let target = temp.path().join("target.txt");
        symlink(&source, &target).unwrap();

        let item = RepoItem {
            source: source.clone(),
            target: target.clone(),
            relative_path: "target.txt".to_string(),
            is_template: false,
        };

        let linker = create_test_linker();
        let result = linker.unlink_item(&item, default_options()).unwrap();

        assert!(matches!(result, LinkResult::Created)); // Created means action taken
        assert!(!target.exists());
    }

    #[test]
    fn test_unlink_item_skips_nonexistent() {
        let temp = TempDir::new().unwrap();
        let source = temp.path().join("source.txt");
        let target = temp.path().join("nonexistent.txt");

        let item = RepoItem {
            source,
            target,
            relative_path: "nonexistent.txt".to_string(),
            is_template: false,
        };

        let linker = create_test_linker();
        let result = linker.unlink_item(&item, default_options()).unwrap();

        assert!(matches!(result, LinkResult::Skipped { .. }));
    }

    #[test]
    fn test_unlink_item_skips_wrong_symlink() {
        let temp = TempDir::new().unwrap();
        let source = temp.path().join("source.txt");
        let other = temp.path().join("other.txt");
        fs::write(&other, "other").unwrap();
        let target = temp.path().join("target.txt");
        symlink(&other, &target).unwrap();

        let item = RepoItem {
            source,
            target: target.clone(),
            relative_path: "target.txt".to_string(),
            is_template: false,
        };

        let linker = create_test_linker();
        let result = linker.unlink_item(&item, default_options()).unwrap();

        assert!(matches!(result, LinkResult::Skipped { .. }));
        assert!(target.exists()); // Should not remove
    }

    // Tests for backup_path

    #[test]
    fn test_backup_path_format() {
        let linker = create_test_linker();
        let path = PathBuf::from("/home/user/.zshrc");
        let backup = linker.backup_path(&path).unwrap();

        let backup_str = backup.to_string_lossy();
        assert!(backup_str.starts_with("/home/user/.zshrc.backup."));
        assert!(backup_str.len() > "/home/user/.zshrc.backup.".len());
    }
}

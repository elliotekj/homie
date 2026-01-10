use anyhow::{bail, Context, Result};
use colored::Colorize;
use std::fs;
use std::os::unix::fs as unix_fs;
use std::path::PathBuf;

use crate::repo::find_repo;

pub fn run(repo_name: &str, file_path: &str, dry_run: bool) -> Result<()> {
    let repo = find_repo(repo_name)?
        .ok_or_else(|| anyhow::anyhow!("Unknown repo: {}", repo_name))?;

    // Expand and resolve the file path
    let expanded = shellexpand::tilde(file_path);
    let source = PathBuf::from(expanded.as_ref())
        .canonicalize()
        .with_context(|| format!("File not found: {}", file_path))?;

    if !source.exists() {
        bail!("File does not exist: {}", source.display());
    }

    if source.is_symlink() {
        bail!("Cannot add symlink: {}", source.display());
    }

    // Determine the relative path from target
    let relative = source
        .strip_prefix(&repo.target)
        .with_context(|| {
            format!(
                "File {} is not under target directory {}",
                source.display(),
                repo.target.display()
            )
        })?;

    // Destination in repo (flat structure - no home/ dir)
    let dest = repo.path.join(relative);

    println!("Adding {} to {}:", file_path.bold(), repo_name.bold());
    println!("  {} -> {}", source.display(), dest.display());

    if dry_run {
        println!("{}", "(dry run - no changes made)".dimmed());
        return Ok(());
    }

    // Create parent directories
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
    }

    // Move file to repo
    fs::rename(&source, &dest)
        .with_context(|| "Failed to move file to repo")?;

    // Create symlink
    unix_fs::symlink(&dest, &source)
        .with_context(|| "Failed to create symlink")?;

    println!("  {} Moved and linked", "✓".green());

    Ok(())
}

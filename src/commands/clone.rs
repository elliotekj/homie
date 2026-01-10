use anyhow::{bail, Context, Result};
use colored::Colorize;
use std::process::Command;

use crate::config::repos_dir;

pub fn run(url: &str, name: Option<&str>, dry_run: bool) -> Result<()> {
    let repos_path = repos_dir()?;
    let repo_name = name.map(String::from).map_or_else(|| extract_repo_name(url), Ok)?;

    let repo_path = repos_path.join(&repo_name);

    if repo_path.exists() {
        bail!("Repo already exists: {}", repo_path.display());
    }

    println!("Cloning {} into {}:", url.bold(), repo_path.display());

    if dry_run {
        println!("  Would run: git clone {} {}", url, repo_path.display());
        println!();
        println!("{}", "(dry run - no changes made)".dimmed());
        return Ok(());
    }

    // Ensure repos directory exists
    if !repos_path.exists() {
        std::fs::create_dir_all(&repos_path)
            .with_context(|| format!("Failed to create: {}", repos_path.display()))?;
    }

    // Run git clone
    let output = Command::new("git")
        .args(["clone", url, &repo_path.to_string_lossy()])
        .output()
        .context("Failed to execute git clone")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git clone failed: {}", stderr.trim());
    }

    println!("  {} Cloned successfully", "✓".green());

    // Check if homie.toml exists
    let config_path = repo_path.join("homie.toml");
    if !config_path.exists() {
        println!();
        println!(
            "{}",
            "Warning: No homie.toml found in cloned repo.".yellow()
        );
        println!("You may need to create one with:");
        println!();
        println!("  target = \"~\"");
        println!();
    } else {
        println!();
        println!("Next steps:");
        println!("  homie link {}", repo_name);
    }

    Ok(())
}

fn extract_repo_name(url: &str) -> Result<String> {
    // Handle various URL formats:
    // git@github.com:user/repo.git
    // https://github.com/user/repo.git
    // https://github.com/user/repo
    // /path/to/repo.git

    let url = url.trim_end_matches('/');
    let url = url.trim_end_matches(".git");

    // Get the last path component
    let name = url
        .rsplit('/')
        .next()
        .or_else(|| url.rsplit(':').next())
        .context("Could not extract repo name from URL")?;

    if name.is_empty() {
        bail!("Could not extract repo name from URL: {}", url);
    }

    Ok(name.to_string())
}

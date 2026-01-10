use anyhow::{bail, Result};
use colored::Colorize;
use std::fs;

use crate::repo::{discover_repos, find_repo, Repo};

pub fn run(repo_name: Option<&str>) -> Result<()> {
    let repos: Vec<Repo> = if let Some(name) = repo_name {
        match find_repo(name)? {
            Some(repo) => vec![repo],
            None => bail!("Unknown repo: {}", name),
        }
    } else {
        discover_repos()?
    };

    if repos.is_empty() {
        println!("No repos found.");
        return Ok(());
    }

    let mut found_diff = false;

    for repo in &repos {
        let items = repo.items()?;
        let mut repo_has_diff = false;

        for item in &items {
            // Skip templates, non-existent targets, and symlinks
            if item.is_template || !item.target.exists() || item.target.is_symlink() {
                continue;
            }

            let (symbol, message) = if item.source.is_file() && item.target.is_file() {
                let source_content = fs::read(&item.source).unwrap_or_default();
                let target_content = fs::read(&item.target).unwrap_or_default();
                if source_content == target_content {
                    continue;
                }
                ("M".yellow(), "modified in target")
            } else {
                ("!".red(), "target is regular file, repo has version")
            };

            if !repo_has_diff {
                println!("{}:", repo.name.bold());
                repo_has_diff = true;
                found_diff = true;
            }
            println!("  {} {} ({})", symbol, item.relative_path, message);
        }

        if repo_has_diff {
            println!();
        }
    }

    if !found_diff {
        println!("No differences found.");
    }

    Ok(())
}

use anyhow::{bail, Context, Result};
use colored::Colorize;
use std::fs;

use crate::config::repos_dir;

pub fn run(name: &str, target: Option<&str>, dry_run: bool) -> Result<()> {
    let repos_path = repos_dir()?;
    let repo_path = repos_path.join(name);

    if repo_path.exists() {
        bail!("Repo already exists: {}", repo_path.display());
    }

    let target_path = target.unwrap_or("~");

    println!(
        "Creating repo {} at {}:",
        name.bold(),
        repo_path.display()
    );

    // Create repos directory if needed
    if !repos_path.exists() {
        println!("  {} Creating {}", "✓".green(), repos_path.display());
        if !dry_run {
            fs::create_dir_all(&repos_path)
                .with_context(|| format!("Failed to create: {}", repos_path.display()))?;
        }
    }

    // Create repo directory
    println!("  {} Creating {}/", "✓".green(), name);
    if !dry_run {
        fs::create_dir_all(&repo_path)
            .with_context(|| format!("Failed to create: {}", repo_path.display()))?;
    }

    // Create homie.toml with target
    let config_content = format!(
        r#"# Homie repo configuration
# Required: where to link files (usually ~ for home directory)
target = "{}"

# Optional: repo-specific variables
# [vars]
# email = "user@example.com"

[defaults]
strategy = "file"  # file | directory | contents

# Override strategy per path
# [strategies]
# ".config/nvim" = "directory"
# ".local/bin" = "contents"

# Paths to ignore (in addition to defaults like .git, homie.toml)
# [ignore]
# paths = ["*.swp", "temp/"]
"#,
        target_path
    );

    println!("  {} Creating homie.toml", "✓".green());
    if !dry_run {
        fs::write(repo_path.join("homie.toml"), config_content)?;
    }

    println!();
    println!("Next steps:");
    println!("  1. Add your dotfiles to {}/", repo_path.display());
    println!("     Example: {}/.zshrc", repo_path.display());
    println!("  2. Run: homie link {}", name);

    if dry_run {
        println!();
        println!("{}", "(dry run - no changes made)".dimmed());
    }

    Ok(())
}

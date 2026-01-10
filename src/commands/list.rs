use anyhow::Result;
use colored::Colorize;

use crate::config::repos_dir;
use crate::repo::discover_repos;

pub fn run() -> Result<()> {
    let repos = discover_repos()?;

    if repos.is_empty() {
        let repos_path = repos_dir()?;
        println!("No repos found in {}", repos_path.display());
        println!();
        println!("To create a new repo:");
        println!("  homie init <name>");
        println!();
        println!("To clone an existing repo:");
        println!("  homie clone <url>");
        return Ok(());
    }

    println!("Repos in ~/.homie/repos/:\n");

    for repo in &repos {
        let items_count = repo.items().map(|i| i.len()).unwrap_or(0);
        let status = format!("{} items", items_count).green();

        println!("  {} {}", repo.name.bold(), format!("({})", status).dimmed());
        println!("    target: {}", repo.target.display());
        if !repo.config.vars.is_empty() {
            println!(
                "    vars:   {}",
                repo.config.vars.keys().cloned().collect::<Vec<_>>().join(", ")
            );
        }
        println!();
    }

    Ok(())
}

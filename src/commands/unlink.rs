use anyhow::{bail, Result};
use colored::Colorize;

use crate::config::GlobalConfig;
use crate::linker::{LinkOptions, LinkResult, Linker};
use crate::repo::{discover_repos, find_repo, Repo};

pub fn run(
    config: &GlobalConfig,
    repo_name: Option<&str>,
    options: LinkOptions,
) -> Result<()> {
    let linker = Linker::new(config.clone());

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

    for repo in &repos {
        println!("{}:", repo.name.bold());

        let items = repo.items()?;

        if items.is_empty() {
            println!("  (no items)");
            continue;
        }

        for item in &items {
            match linker.unlink_item(item, options) {
                Ok(result) => {
                    match result {
                        LinkResult::Created => {
                            println!("  {} {}", "✓".green(), item.relative_path);
                        }
                        LinkResult::Skipped { reason } => {
                            if options.verbose {
                                println!(
                                    "  {} {} ({})",
                                    "⊘".yellow(),
                                    item.relative_path,
                                    reason.dimmed()
                                );
                            }
                        }
                        _ => {}
                    }
                }
                Err(e) => {
                    println!("  {} {} ({})", "✗".red(), item.relative_path, e);
                }
            }
        }

        println!();
    }

    if options.dry_run {
        println!("{}", "(dry run - no changes made)".dimmed());
    }

    Ok(())
}

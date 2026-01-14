use anyhow::{bail, Result};
use colored::Colorize;

use crate::config::GlobalConfig;
use crate::linker::{LinkOptions, LinkResult, Linker};
use crate::manifest::Manifest;
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

        let manifest = Manifest::load(&repo.path).unwrap_or_default();

        if manifest.is_empty() {
            let items = repo.items()?;
            if items.is_empty() {
                println!("  (no items)");
                continue;
            }

            for item in &items {
                let result = linker.unlink_item(item, options);
                print_unlink_result(&item.relative_path, result, options.verbose);
            }
        } else {
            for (path, entry) in manifest.iter() {
                let target = repo.target.join(path);
                let result = linker.unlink_from_manifest(&target, *entry, options);
                print_unlink_result(path, result, options.verbose);
            }

            if !options.dry_run {
                Manifest::default().save(&repo.path)?;
            }
        }

        println!();
    }

    if options.dry_run {
        println!("{}", "(dry run - no changes made)".dimmed());
    }

    Ok(())
}

fn print_unlink_result(path: &str, result: Result<LinkResult>, verbose: bool) {
    match result {
        Ok(LinkResult::Unlinked) => {
            println!("  {} {}", "✓".green(), path);
        }
        Ok(LinkResult::Skipped { reason }) => {
            if verbose {
                println!("  {} {} ({})", "⊘".yellow(), path, reason.dimmed());
            }
        }
        Ok(LinkResult::Created { .. })
        | Ok(LinkResult::AlreadyCorrect { .. })
        | Ok(LinkResult::BackedUp { .. }) => {}
        Err(e) => {
            println!("  {} {} ({})", "✗".red(), path, e);
        }
    }
}

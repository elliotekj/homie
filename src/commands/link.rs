use anyhow::{bail, Result};
use colored::Colorize;

use crate::config::GlobalConfig;
use crate::linker::{print_result, LinkOptions, LinkResult, Linker};
use crate::manifest::Manifest;
use crate::repo::{discover_repos, find_repo, Repo};
use crate::vars::VarResolver;

pub fn run(
    config: &GlobalConfig,
    repo_name: Option<&str>,
    options: LinkOptions,
) -> Result<()> {
    let linker = Linker::new(config.clone());

    let repos: Vec<Repo> = if let Some(name) = repo_name {
        match find_repo(name)? {
            Some(repo) => vec![repo],
            None => bail!("Unknown repo: {}. Run 'homie list' to see available repos.", name),
        }
    } else {
        discover_repos()?
    };

    if repos.is_empty() {
        println!("No repos found in ~/.homie/repos/");
        println!("Run 'homie init <name>' to create a new repo.");
        return Ok(());
    }

    for repo in &repos {
        println!("{}:", repo.name.bold());

        if !options.no_fetch && !repo.imports.is_empty() {
            repo.fetch_imports(options.dry_run)?;
        }

        let var_resolver = VarResolver::new(config, Some(repo));
        let items = repo.items()?;

        if items.is_empty() {
            println!("  (no items to link)");
            continue;
        }

        let mut manifest = Manifest::default();

        for item in &items {
            match linker.link_item(item, &var_resolver, &repo.path, options) {
                Ok(result) => {
                    print_result(&item.relative_path, &result, options.verbose);

                    let entry = match &result {
                        LinkResult::Created { entry } => Some(*entry),
                        LinkResult::AlreadyCorrect { entry } => Some(*entry),
                        LinkResult::BackedUp { entry, .. } => Some(*entry),
                        LinkResult::Skipped { .. } => None,
                        LinkResult::Unlinked => None,
                    };
                    if let Some(entry) = entry {
                        manifest.insert(item.relative_path.clone(), entry);
                    }
                }
                Err(e) => {
                    println!("  {} {} ({})", "✗".red(), item.relative_path, e);
                }
            }
        }

        if !options.dry_run && !manifest.is_empty() {
            manifest.save(&repo.path)?;
        }

        println!();
    }

    if options.dry_run {
        println!("{}", "(dry run - no changes made)".dimmed());
    }

    Ok(())
}

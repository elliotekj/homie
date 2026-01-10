use anyhow::{bail, Result};
use colored::Colorize;

use crate::config::GlobalConfig;
use crate::linker::{print_result, LinkOptions, Linker};
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

        // Fetch git imports if needed
        if !options.no_fetch && !repo.imports.is_empty() {
            repo.fetch_imports(options.dry_run)?;
        }

        let var_resolver = VarResolver::new(config, Some(repo));
        let items = repo.items()?;

        if items.is_empty() {
            println!("  (no items to link)");
            continue;
        }

        for item in &items {
            match linker.link_item(item, &var_resolver, &repo.path, options) {
                Ok(result) => {
                    print_result(&item.relative_path, &result, options.verbose);
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

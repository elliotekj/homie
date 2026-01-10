use anyhow::{bail, Result};
use colored::Colorize;

use crate::repo::{discover_repos, find_repo, Repo};
use crate::status::{check_item_status, ItemStatus, RepoStatus};

pub fn run(repo_name: Option<&str>, verbose: bool) -> Result<()> {
    let repos: Vec<Repo> = if let Some(name) = repo_name {
        match find_repo(name)? {
            Some(repo) => vec![repo],
            None => bail!("Unknown repo: {}", name),
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
        let items = repo.items()?;
        let mut status = RepoStatus::default();

        for item in &items {
            let item_status = check_item_status(item, &repo.path);

            match &item_status {
                ItemStatus::Linked => status.linked += 1,
                ItemStatus::External(_) => status.external += 1,
                ItemStatus::Missing => status.missing += 1,
                ItemStatus::Conflict => status.conflict += 1,
                ItemStatus::Rendered => status.rendered += 1,
            }

            if verbose {
                let symbol = match &item_status {
                    ItemStatus::Linked => "✓".green(),
                    ItemStatus::External(_) => "⊘".yellow(),
                    ItemStatus::Missing => "?".red(),
                    ItemStatus::Conflict => "!".red(),
                    ItemStatus::Rendered => "✓".cyan(),
                };
                let note = match &item_status {
                    ItemStatus::External(path) => format!(" (external: {})", path),
                    ItemStatus::Rendered => " (rendered)".to_string(),
                    _ => String::new(),
                };
                println!("  {} {}{}", symbol, item.relative_path, note.dimmed());
            }
        }

        println!(
            "{} ({} items):",
            repo.name.bold(),
            status.total()
        );
        println!("  linked:   {}", format_count(status.linked, StatusColor::Green));
        if status.rendered > 0 {
            println!("  rendered: {}", format_count(status.rendered, StatusColor::Cyan));
        }
        if status.external > 0 {
            println!(
                "  external: {}  (preserved, pointing outside repos)",
                format_count(status.external, StatusColor::Yellow)
            );
        }
        if status.missing > 0 {
            println!(
                "  missing:  {}  (in repo but not linked)",
                format_count(status.missing, StatusColor::Red)
            );
        }
        if status.conflict > 0 {
            println!(
                "  conflict: {}  (file exists, not a symlink)",
                format_count(status.conflict, StatusColor::Red)
            );
        }
        println!();
    }

    Ok(())
}

enum StatusColor {
    Green,
    Yellow,
    Red,
    Cyan,
}

fn format_count(count: usize, color: StatusColor) -> String {
    let s = count.to_string();
    match color {
        StatusColor::Green => s.green().to_string(),
        StatusColor::Yellow => s.yellow().to_string(),
        StatusColor::Red => s.red().to_string(),
        StatusColor::Cyan => s.cyan().to_string(),
    }
}

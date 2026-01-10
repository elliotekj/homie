use std::fs;
use std::path::Path;

use crate::repo::RepoItem;

#[derive(Debug, Default)]
pub struct RepoStatus {
    pub linked: usize,
    pub external: usize,
    pub missing: usize,
    pub conflict: usize,
    pub rendered: usize,
}

#[derive(Debug)]
pub enum ItemStatus {
    Linked,
    External(String),
    Missing,
    Conflict,
    Rendered,
}

impl RepoStatus {
    pub fn total(&self) -> usize {
        self.linked + self.external + self.missing + self.conflict + self.rendered
    }
}

pub fn check_item_status(item: &RepoItem, repo_path: &Path) -> ItemStatus {
    // Templates are rendered files, not symlinks
    if item.is_template {
        return if item.target.exists() {
            ItemStatus::Rendered
        } else {
            ItemStatus::Missing
        };
    }

    if !item.target.exists() && !item.target.is_symlink() {
        return ItemStatus::Missing;
    }

    if !item.target.is_symlink() {
        return ItemStatus::Conflict;
    }

    let Ok(link_target) = fs::read_link(&item.target) else {
        return ItemStatus::Conflict;
    };

    let resolved = if link_target.is_absolute() {
        link_target
    } else {
        item.target.parent().unwrap_or(Path::new("")).join(&link_target)
    };

    if resolved == item.source || resolved.starts_with(repo_path) {
        ItemStatus::Linked
    } else {
        ItemStatus::External(resolved.to_string_lossy().to_string())
    }
}

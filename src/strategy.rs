use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Strategy {
    #[default]
    File,
    Directory,
    Contents,
    Copy,
}

impl Strategy {
    pub fn is_copy(&self) -> bool {
        match self {
            Strategy::File => false,
            Strategy::Directory => false,
            Strategy::Contents => false,
            Strategy::Copy => true,
        }
    }

    pub fn is_directory_unit(&self) -> bool {
        match self {
            Strategy::File => false,
            Strategy::Directory => true,
            Strategy::Contents => false,
            Strategy::Copy => true,
        }
    }
}

impl std::fmt::Display for Strategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Strategy::File => "file",
            Strategy::Directory => "directory",
            Strategy::Contents => "contents",
            Strategy::Copy => "copy",
        };
        write!(f, "{}", s)
    }
}

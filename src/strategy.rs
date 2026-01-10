use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Strategy {
    #[default]
    File,
    Directory,
    Contents,
}

impl Strategy {
    pub fn as_str(&self) -> &'static str {
        match self {
            Strategy::File => "file",
            Strategy::Directory => "directory",
            Strategy::Contents => "contents",
        }
    }
}

impl std::fmt::Display for Strategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

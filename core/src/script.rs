use std::path::PathBuf;

use crate::MainstageErrorExt;

#[derive(Debug, Clone)]
pub struct Script {
    pub name: String,
    pub path: PathBuf,
    pub content: String,
}

impl Script {
    pub fn new(path: PathBuf) -> Result<Self, Box<dyn MainstageErrorExt>> {
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let content = std::fs::read_to_string(&path).map_err(|_| {
            Box::<dyn MainstageErrorExt>::from(Box::new(MissingScriptError { path: path.clone() }))
        })?;
        Ok(Script {
            name,
            path,
            content,
        })
    }

    pub fn is_empty(&self) -> bool {
        self.content.trim().is_empty()
    }

    pub fn display_content(&self) -> &str {
        &self.content
    }
}

impl std::fmt::Display for Script {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Script: {} at {:?}", self.name, self.path)
    }
}

#[derive(Debug, Clone)]
pub struct MissingScriptError {
    pub path: PathBuf,
}

impl std::fmt::Display for MissingScriptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Missing script at {:?}", self.path)
    }
}

impl std::error::Error for MissingScriptError {}

impl MainstageErrorExt for MissingScriptError {
    fn level(&self) -> crate::Level {
        crate::Level::Error
    }

    fn message(&self) -> String {
        format!("Missing script at {:?}", self.path)
    }

    fn issuer(&self) -> String {
        "mainstage.script".to_string()
    }

    fn span(&self) -> Option<crate::location::Span> {
        None
    }

    fn location(&self) -> Option<crate::location::Location> {
        None
    }
}

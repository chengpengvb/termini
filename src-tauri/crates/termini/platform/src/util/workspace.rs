use anyhow::Result;
use std::path::PathBuf;
use crate::util::ios_local_dir;

pub fn workspace_dir() -> Result<PathBuf> {
    Ok(ios_local_dir::documents_dir()?.join("Termini").join("Workspace"))
}

pub fn deck_dir() -> Result<PathBuf> {
    Ok(workspace_dir()?.join("Deck"))
}

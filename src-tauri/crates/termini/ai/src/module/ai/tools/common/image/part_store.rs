use std::path::PathBuf;
use serde::{Deserialize, Serialize};

const PARTS_DIR: &str = "parts";
const MANIFEST_FILE: &str = "manifest.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Manifest {
    source_hash: String,
    total: usize,
}

pub struct PartStore {
    dir: PathBuf,
    source_hash: String,
    total: usize,
}

impl PartStore {
    pub fn open(derived_dir: PathBuf, variant: &str, source_hash: &str, total: usize) -> PartStore {
        PartStore { dir: derived_dir.join(PARTS_DIR).join(variant), source_hash: source_hash.to_string(), total }
    }

    pub fn load(&self) -> Vec<Option<String>> {
        let mut parts = vec![None; self.total];
        let manifest_path = self.dir.join(MANIFEST_FILE);
        let Ok(raw) = std::fs::read_to_string(&manifest_path) else {
            return parts;
        };
        let Ok(manifest) = serde_json::from_str::<Manifest>(&raw) else {
            self.clear();
            return parts;
        };
        if manifest.source_hash != self.source_hash || manifest.total != self.total {
            self.clear();
            return parts;
        }
        for (index, slot) in parts.iter_mut().enumerate() {
            if let Ok(text) = std::fs::read_to_string(self.part_path(index)) {
                *slot = Some(text);
            }
        }
        parts
    }

    pub fn save(&self, index: usize, text: &str) -> Result<(), String> {
        std::fs::create_dir_all(&self.dir).map_err(|error| format!("create {}: {}", self.dir.display(), error))?;
        let manifest_path = self.dir.join(MANIFEST_FILE);
        if !manifest_path.is_file() {
            let manifest = Manifest { source_hash: self.source_hash.clone(), total: self.total };
            let raw = serde_json::to_string(&manifest).map_err(|error| error.to_string())?;
            std::fs::write(&manifest_path, raw).map_err(|error| format!("write {}: {}", manifest_path.display(), error))?;
        }
        let path = self.part_path(index);
        let temp = path.with_extension("tmp");
        std::fs::write(&temp, text).map_err(|error| format!("write {}: {}", temp.display(), error))?;
        std::fs::rename(&temp, &path).map_err(|error| format!("rename {}: {}", path.display(), error))?;
        Ok(())
    }

    pub fn clear(&self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }

    fn part_path(&self, index: usize) -> PathBuf {
        self.dir.join(format!("{:04}.txt", index))
    }
}

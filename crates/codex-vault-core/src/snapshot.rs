use crate::utils::{path_to_string, relative_path};
use anyhow::{Context, Result};
use glob::glob;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

#[derive(Debug)]
pub struct Snapshot {
    pub source_home: PathBuf,
    pub root: PathBuf,
    pub warnings: Vec<String>,
    _temp_dir: TempDir,
}

impl Snapshot {
    pub fn create(source_home: &Path) -> Result<Self> {
        let temp_dir = tempfile::Builder::new()
            .prefix("codex-recall-snapshot-")
            .tempdir()
            .context("create temporary snapshot directory")?;

        let root = temp_dir.path().to_path_buf();
        let mut snapshot = Self {
            source_home: source_home.to_path_buf(),
            root,
            warnings: Vec::new(),
            _temp_dir: temp_dir,
        };

        snapshot.copy_if_present("session_index.jsonl");
        snapshot.copy_if_present(".codex-global-state.json");
        snapshot.copy_glob("state_*.sqlite");
        snapshot.copy_glob("state_*.sqlite-wal");
        snapshot.copy_glob("state_*.sqlite-shm");
        snapshot.copy_glob(".codex-global-state.json.bak*");

        Ok(snapshot)
    }

    pub fn source_path(&self, relative: &str) -> PathBuf {
        self.source_home.join(relative)
    }

    pub fn snapshot_path(&self, relative: &str) -> PathBuf {
        self.root.join(relative)
    }

    pub fn find_state_databases(&self) -> Vec<PathBuf> {
        let pattern = self.root.join("state_*.sqlite");
        glob(pattern.to_string_lossy().as_ref())
            .map(|matches| {
                let mut paths = matches.flatten().collect::<Vec<_>>();
                paths.sort();
                paths
            })
            .unwrap_or_default()
    }

    fn copy_if_present(&mut self, relative: &str) {
        let source = self.source_path(relative);
        if source.is_file() {
            self.copy_file(&source, relative);
        }
    }

    fn copy_glob(&mut self, pattern: &str) {
        let full_pattern = self.source_home.join(pattern);
        let Some(pattern) = full_pattern.to_str() else {
            self.warnings.push(format!(
                "snapshot pattern is not valid UTF-8: {}",
                path_to_string(&full_pattern)
            ));
            return;
        };

        match glob(pattern) {
            Ok(matches) => {
                for entry in matches {
                    match entry {
                        Ok(path) if path.is_file() => {
                            let relative = relative_path(&self.source_home, &path);
                            self.copy_file(&path, &relative);
                        }
                        Ok(_) => {}
                        Err(error) => self.warnings.push(format!("glob entry error: {error}")),
                    }
                }
            }
            Err(error) => self
                .warnings
                .push(format!("glob pattern error for {pattern}: {error}")),
        }
    }

    fn copy_file(&mut self, source: &Path, relative: &str) {
        let dest = self.snapshot_path(relative);
        if let Some(parent) = dest.parent() {
            if let Err(error) = fs::create_dir_all(parent) {
                self.warnings.push(format!(
                    "failed to create snapshot dir for {}: {error}",
                    path_to_string(source)
                ));
                return;
            }
        }

        if let Err(error) = fs::copy(source, &dest) {
            self.warnings.push(format!(
                "failed to copy {} into snapshot: {error}",
                path_to_string(source)
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn snapshots_metadata_without_copying_sessions_tree() {
        let temp = tempdir().unwrap();
        fs::create_dir(temp.path().join("sessions")).unwrap();
        fs::write(temp.path().join("sessions").join("a.jsonl"), "{}\n").unwrap();
        fs::write(temp.path().join("session_index.jsonl"), "{}\n").unwrap();
        fs::write(temp.path().join("state_5.sqlite"), b"not really sqlite").unwrap();

        let snapshot = Snapshot::create(temp.path()).unwrap();
        assert!(snapshot.snapshot_path("session_index.jsonl").is_file());
        assert!(snapshot.snapshot_path("state_5.sqlite").is_file());
        assert!(!snapshot.snapshot_path("sessions/a.jsonl").exists());
    }
}

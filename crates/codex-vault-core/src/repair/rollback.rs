use crate::models::{RollbackFile, RollbackPackage, RollbackResult};
use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub fn create_rollback_package(
    output_dir: &Path,
    full_backup_path: Option<String>,
    files: &[PathBuf],
) -> Result<RollbackPackage> {
    let id = Uuid::new_v4().to_string();
    let root = output_dir.join(format!("rollback-{id}"));
    let files_root = root.join("files");
    fs::create_dir_all(&files_root)
        .with_context(|| format!("create rollback directory {}", files_root.display()))?;

    let mut rollback_files = Vec::new();
    for source in files {
        if !source.is_file() {
            continue;
        }
        let backup_path = files_root.join(safe_name(source));
        fs::copy(source, &backup_path).with_context(|| {
            format!(
                "copy rollback source {} to {}",
                source.display(),
                backup_path.display()
            )
        })?;
        rollback_files.push(RollbackFile {
            original_path: source.to_string_lossy().to_string(),
            backup_path: backup_path.to_string_lossy().to_string(),
            sha256: sha256_file(&backup_path)?,
        });
    }

    let manifest_path = root.join("rollback-manifest.json");
    let package = RollbackPackage {
        id,
        created_at: Utc::now(),
        root_path: root.to_string_lossy().to_string(),
        manifest_path: manifest_path.to_string_lossy().to_string(),
        full_backup_path,
        auto_restore_supported: !rollback_files.is_empty(),
        files: rollback_files,
    };
    fs::write(&manifest_path, serde_json::to_string_pretty(&package)?)
        .with_context(|| format!("write rollback manifest {}", manifest_path.display()))?;
    Ok(package)
}

pub fn restore_rollback_package(
    package: &RollbackPackage,
    confirm_restore: bool,
) -> Result<RollbackResult> {
    if !confirm_restore {
        return Err(anyhow!("Rollback confirmation is required."));
    }
    if package.files.is_empty() {
        return Ok(RollbackResult {
            rollback_id: package.id.clone(),
            restored_files: Vec::new(),
            succeeded: false,
            warnings: vec!["This package only references a full backup; automatic file restore is not supported.".to_string()],
            errors: Vec::new(),
        });
    }

    let mut restored = Vec::new();
    let mut errors = Vec::new();
    for file in &package.files {
        let backup = Path::new(&file.backup_path);
        match sha256_file(backup) {
            Ok(hash) if hash == file.sha256 => {
                let original = Path::new(&file.original_path);
                if let Some(parent) = original.parent() {
                    if let Err(error) = fs::create_dir_all(parent) {
                        errors.push(error.to_string());
                        continue;
                    }
                }
                match fs::copy(backup, original) {
                    Ok(_) => restored.push(file.original_path.clone()),
                    Err(error) => errors.push(error.to_string()),
                }
            }
            Ok(_) => errors.push(format!(
                "Rollback backup hash mismatch: {}",
                file.backup_path
            )),
            Err(error) => errors.push(error.to_string()),
        }
    }

    Ok(RollbackResult {
        rollback_id: package.id.clone(),
        restored_files: restored,
        succeeded: errors.is_empty(),
        warnings: Vec::new(),
        errors,
    })
}

fn safe_name(path: &Path) -> String {
    let mut hash = Sha256::new();
    hash.update(path.to_string_lossy().as_bytes());
    let digest = format!("{:x}", hash.finalize());
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("file");
    format!("{}-{}", &digest[..16], name.replace('/', "_"))
}

fn sha256_file(path: &Path) -> Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

#[allow(dead_code)]
fn manifest_preview(package: &RollbackPackage) -> serde_json::Value {
    json!({
        "id": package.id,
        "files": package.files.len(),
        "full_backup_path": package.full_backup_path,
        "auto_restore_supported": package.auto_restore_supported
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn rollback_restores_file_copy() {
        let temp = tempdir().unwrap();
        let source = temp.path().join("state.json");
        fs::write(&source, "before").unwrap();
        let package = create_rollback_package(temp.path(), None, &[source.clone()]).unwrap();
        fs::write(&source, "after").unwrap();
        let result = restore_rollback_package(&package, true).unwrap();
        assert!(result.succeeded);
        assert_eq!(fs::read_to_string(source).unwrap(), "before");
    }
}

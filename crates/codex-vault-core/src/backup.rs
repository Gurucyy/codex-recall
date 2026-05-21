use crate::models::{BackupManifest, BackupManifestFile, BackupRequest, BackupResult};
use crate::utils::{ensure_parent_dir, file_mtime, path_to_string, relative_path};
use anyhow::{Context, Result};
use chrono::Utc;
use glob::Pattern;
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use zip::write::FileOptions;

pub fn backup_codex_home(request: &BackupRequest) -> Result<BackupResult> {
    let codex_home = PathBuf::from(&request.codex_home);
    let output_path = PathBuf::from(&request.output_path);
    ensure_parent_dir(&output_path)?;
    let temp_path = output_path.with_extension(format!(
        "{}tmp",
        output_path
            .extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| format!("{extension}."))
            .unwrap_or_default()
    ));

    let files = collect_backup_files(&codex_home)?;
    let mut manifest_files = Vec::new();
    let mut total_bytes = 0u64;
    let created_at = Utc::now();

    {
        let file = File::create(&temp_path)
            .with_context(|| format!("create temporary backup {}", temp_path.display()))?;
        let mut zip = zip::ZipWriter::new(file);
        let options = FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        for source in &files {
            let relative = relative_path(&codex_home, source);
            let metadata = fs::metadata(source)?;
            let sha256 = sha256_file(source)?;
            total_bytes += metadata.len();
            manifest_files.push(BackupManifestFile {
                path: relative.clone(),
                size: metadata.len(),
                sha256,
                mtime: file_mtime(source),
            });

            zip.start_file(relative.replace('\\', "/"), options)?;
            let mut input = File::open(source)?;
            std::io::copy(&mut input, &mut zip)?;
        }

        let manifest = BackupManifest {
            source_codex_home: path_to_string(&codex_home),
            created_at,
            files: manifest_files.clone(),
        };
        zip.start_file("backup-manifest.json", options)?;
        zip.write_all(serde_json::to_string_pretty(&manifest)?.as_bytes())?;
        zip.finish()?;
    }

    fs::rename(&temp_path, &output_path).with_context(|| {
        format!(
            "rename temporary backup {} to {}",
            temp_path.display(),
            output_path.display()
        )
    })?;

    let manifest = BackupManifest {
        source_codex_home: path_to_string(&codex_home),
        created_at,
        files: manifest_files,
    };
    Ok(BackupResult {
        output_path: output_path.to_string_lossy().to_string(),
        total_files: manifest.files.len(),
        total_bytes,
        manifest,
        warnings: Vec::new(),
    })
}

fn collect_backup_files(codex_home: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for root in ["sessions", "archived_sessions"] {
        let path = codex_home.join(root);
        if path.exists() {
            for entry in WalkDir::new(&path)
                .into_iter()
                .filter_map(|entry| entry.ok())
            {
                if entry.file_type().is_file() {
                    files.push(entry.into_path());
                }
            }
        }
    }

    let Ok(entries) = fs::read_dir(codex_home) else {
        return Ok(files);
    };
    for entry in entries {
        let Ok(entry) = entry else {
            continue;
        };
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if is_allowed_root_file(file_name) {
            files.push(path);
        }
    }

    files.sort();
    files.dedup();
    Ok(files)
}

fn is_allowed_root_file(file_name: &str) -> bool {
    file_name == "session_index.jsonl"
        || file_name == ".codex-global-state.json"
        || Pattern::new("state_*.sqlite")
            .map(|pattern| pattern.matches(file_name))
            .unwrap_or(false)
        || Pattern::new("state_*.sqlite-wal")
            .map(|pattern| pattern.matches(file_name))
            .unwrap_or(false)
        || Pattern::new("state_*.sqlite-shm")
            .map(|pattern| pattern.matches(file_name))
            .unwrap_or(false)
        || Pattern::new(".codex-global-state.json.bak*")
            .map(|pattern| pattern.matches(file_name))
            .unwrap_or(false)
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn backup_includes_manifest_and_allowed_files() {
        let temp = tempdir().unwrap();
        fs::create_dir(temp.path().join("sessions")).unwrap();
        fs::write(temp.path().join("sessions").join("a.jsonl"), "{}\n").unwrap();
        fs::write(temp.path().join("state_5.sqlite"), "sqlite").unwrap();
        fs::write(temp.path().join("ignore.txt"), "ignore").unwrap();
        let output = temp.path().join("backup.zip");

        let result = backup_codex_home(&BackupRequest {
            codex_home: temp.path().to_string_lossy().to_string(),
            output_path: output.to_string_lossy().to_string(),
        })
        .unwrap();

        assert!(output.is_file());
        assert_eq!(result.total_files, 2);
        assert!(result
            .manifest
            .files
            .iter()
            .any(|file| file.path == "sessions/a.jsonl"));
        assert!(!result
            .manifest
            .files
            .iter()
            .any(|file| file.path == "ignore.txt"));
    }
}

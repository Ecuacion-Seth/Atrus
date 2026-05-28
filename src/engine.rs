use crate::config::AppConfig;
use crate::models::{ChangeKind, FileItem, PendingChange, RenameCase, UndoLog};
use anyhow::Result;
use chrono::{DateTime, Local};
use regex::Regex;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub fn read_directory(path: &Path) -> Result<Vec<FileItem>> {
    let mut files = Vec::new();
    let entries = fs::read_dir(path)?;

    for entry in entries {
        let entry = entry?;
        let metadata = entry.metadata()?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        let size = metadata.len();
        let is_dir = metadata.is_dir();

        let permissions = if metadata.permissions().readonly() {
            "r--".to_string()
        } else {
            "rw-".to_string()
        };

        let modified: DateTime<Local> = metadata.modified()?.into();
        let modified_str = modified.format("%Y-%m-%d %H:%M").to_string();

        files.push(FileItem {
            name,
            path,
            size,
            is_dir,
            permissions,
            modified: modified_str,
        });
    }

    files.sort_by(|a, b| {
        if a.is_dir && !b.is_dir {
            std::cmp::Ordering::Less
        } else if !a.is_dir && b.is_dir {
            std::cmp::Ordering::Greater
        } else {
            a.name.cmp(&b.name)
        }
    });

    Ok(files)
}

pub fn generate_sort_changes(current_dir: &Path, config: &AppConfig) -> Result<Vec<PendingChange>> {
    let mut changes = Vec::new();
    let entries = fs::read_dir(current_dir)?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                let ext_lower = ext.to_lowercase();
                let category = config
                    .extension_map
                    .get(&ext_lower)
                    .cloned()
                    .unwrap_or_else(|| "Others".to_string());

                let target_dir = config
                    .sort_targets
                    .get(&category)
                    .or_else(|| config.sort_targets.get("Others"));

                if let Some(target_dir) = target_dir {
                    let resolved_target = if target_dir.is_relative() {
                        current_dir.join(target_dir)
                    } else {
                        target_dir.clone()
                    };

                    let dest = resolved_target.join(path.file_name().unwrap());

                    changes.push(PendingChange {
                        description: format!("Sort file {} -> {}", path.display(), dest.display()),
                        src: path.clone(),
                        dst: dest,
                        kind: ChangeKind::Move,
                    });
                }
            }
        }
    }
    Ok(changes)
}

pub fn generate_cleanup_changes(
    current_dir: &Path,
    _config: &AppConfig,
) -> Result<Vec<PendingChange>> {
    let mut changes = Vec::new();
    for entry in WalkDir::new(current_dir).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_dir() {
            if let Ok(mut read_dir) = fs::read_dir(path) {
                if read_dir.next().is_none() && path != current_dir {
                    changes.push(PendingChange {
                        description: format!(
                            "Delete empty target directory layout node: {}",
                            path.display()
                        ),
                        src: path.to_path_buf(),
                        dst: PathBuf::new(),
                        kind: ChangeKind::DeleteDir,
                    });
                }
            }
        }
    }
    Ok(changes)
}

pub fn find_large_files(current_dir: &Path, max_count: usize) -> Result<Vec<(PathBuf, u64)>> {
    let mut files = Vec::new();
    for entry in WalkDir::new(current_dir).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_file() {
            if let Ok(meta) = path.metadata() {
                files.push((path.to_path_buf(), meta.len()));
            }
        }
    }
    files.sort_by(|a, b| b.1.cmp(&a.1));
    files.truncate(max_count);
    Ok(files)
}

pub fn find_duplicate_files(current_dir: &Path) -> Result<Vec<Vec<PathBuf>>> {
    let mut hashes: HashMap<String, Vec<PathBuf>> = HashMap::new();

    for entry in WalkDir::new(current_dir).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_file() {
            if let Ok(hash) = compute_file_hash(path) {
                hashes.entry(hash).or_default().push(path.to_path_buf());
            }
        }
    }

    let duplicates = hashes
        .into_values()
        .filter(|paths| paths.len() > 1)
        .collect();
    Ok(duplicates)
}

fn compute_file_hash(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0; 4096];

    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

pub fn batch_rename_dry_run(
    current_dir: &Path,
    prefix: &str,
    case: &RenameCase,
) -> Result<Vec<PendingChange>> {
    let mut changes = Vec::new();
    let entries = fs::read_dir(current_dir)?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            let stem = path.file_stem().unwrap().to_string_lossy();
            let extension = path
                .extension()
                .map(|e| format!(".{}", e.to_string_lossy()))
                .unwrap_or_default();

            let modulated_stem = match case {
                RenameCase::Lowercase => stem.to_lowercase(),
                RenameCase::Uppercase => stem.to_uppercase(),
                RenameCase::CamelCase => to_camel_case(&stem),
            };

            let new_name = format!("{}{}{}", prefix, modulated_stem, extension);
            let dest = path.with_file_name(new_name);

            changes.push(PendingChange {
                description: format!(
                    "Rename transformation: {} -> {}",
                    path.file_name().unwrap().to_string_lossy(),
                    dest.file_name().unwrap().to_string_lossy()
                ),
                src: path.clone(),
                dst: dest,
                kind: ChangeKind::Move,
            });
        }
    }

    Ok(changes)
}

fn to_camel_case(s: &str) -> String {
    s.split(|c: char| c.is_whitespace() || c == '_' || c == '-')
        .enumerate()
        .map(|(i, word)| {
            if i == 0 {
                word.to_lowercase()
            } else {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => {
                        first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase()
                    }
                }
            }
        })
        .collect()
}

pub fn apply_changes(changes: &[PendingChange]) -> anyhow::Result<(Vec<UndoLog>, Vec<String>)> {
    let mut undo_logs = Vec::new();
    let mut errors = Vec::new(); // Collect non-fatal errors

    for change in changes {
        match change.kind {
            ChangeKind::Move => {
                if let Some(parent) = change.dst.parent() {
                    if !parent.exists() {
                        if let Err(e) = fs::create_dir_all(parent) {
                            errors.push(format!(
                                "Could not create directory '{}': {}",
                                parent.display(),
                                e
                            ));
                            continue;
                        }
                    }
                }

                if fs::rename(&change.src, &change.dst).is_err() {
                    if fs::copy(&change.src, &change.dst).is_ok() {
                        let _ = fs::remove_file(&change.src);
                    } else {
                        errors.push(format!("Failed to move: {}", change.src.display()));
                        continue;
                    }
                }

                undo_logs.push(UndoLog {
                    old_path: change.src.clone(),
                    new_path: change.dst.clone(),
                });
            }
            ChangeKind::DeleteDir => {
                if change.src.exists() {
                    if let Err(e) = fs::remove_dir(&change.src) {
                        // Log the error but don't crash the loop
                        errors.push(format!(
                            "Could not remove '{}': {}",
                            change.src.display(),
                            e
                        ));
                    }
                }
            }
        }
    }

    Ok((undo_logs, errors))
}

pub fn undo_operations(logs: &[UndoLog]) -> Result<()> {
    for log in logs.iter().rev() {
        if log.new_path.exists() {
            fs::rename(&log.new_path, &log.old_path)?;
        }
    }
    Ok(())
}

pub fn filter_files(files: &[FileItem], query: &str) -> anyhow::Result<(Vec<FileItem>, bool)> {
    match Regex::new(query) {
        Ok(regex) => Ok((
            files
                .iter()
                .filter(|f| regex.is_match(&f.name))
                .cloned()
                .collect(),
            false, // No fallback used
        )),
        Err(_) => {
            // Fallback to case-insensitive plain text search on invalid regex
            let lower_query = query.to_lowercase();
            Ok((
                files
                    .iter()
                    .filter(|f| f.name.to_lowercase().contains(&lower_query))
                    .cloned()
                    .collect(),
                true, // Fallback triggered
            ))
        }
    }
}

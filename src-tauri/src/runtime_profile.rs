use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

use serde::Serialize;
use uuid::Uuid;

use crate::error::AppError;

pub const PORTABLE_MARKER: &str = ".sorttrack-portable";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum StorageMode {
    Installed,
    Portable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeProfile {
    pub mode: StorageMode,
    pub data_root: PathBuf,
}

pub fn select_installed_data_root(preferred: &Path, legacy: &Path) -> Result<PathBuf, AppError> {
    let preferred_database = preferred.join("database").join("sorttrack.db");
    let legacy_database = legacy.join("database").join("sorttrack.db");
    let preferred_exists = preferred_database.try_exists()?;
    let legacy_exists = legacy_database.try_exists()?;
    match (preferred_exists, legacy_exists) {
        (true, true) => Err(AppError::Task(format!(
            "Обнаружены две базы установленной версии: {} и {}. SortTrack не будет выбирать одну автоматически; создайте резервные копии и оставьте одну базу.",
            preferred_database.display(),
            legacy_database.display()
        ))),
        (false, true) => Ok(legacy.to_path_buf()),
        _ => Ok(preferred.to_path_buf()),
    }
}

pub fn resolve(installed_data_root: &Path, executable: &Path) -> Result<RuntimeProfile, AppError> {
    let executable_directory = executable.parent().ok_or_else(|| {
        AppError::Task("Не удалось определить папку исполняемого файла SortTrack".into())
    })?;
    let marker = executable_directory.join(PORTABLE_MARKER);
    let mode = match fs::symlink_metadata(&marker) {
        Ok(metadata) if metadata.file_type().is_file() && !metadata.file_type().is_symlink() => {
            StorageMode::Portable
        }
        Ok(_) => {
            return Err(AppError::Task(format!(
                "Маркер portable-режима имеет недопустимый тип: {}",
                marker.display()
            )));
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => StorageMode::Installed,
        Err(error) => return Err(error.into()),
    };
    let requested_root = match mode {
        StorageMode::Installed => installed_data_root.to_path_buf(),
        StorageMode::Portable => executable_directory.join("data"),
    };
    ensure_writable(&requested_root, mode)?;
    Ok(RuntimeProfile {
        mode,
        data_root: requested_root.canonicalize()?,
    })
}

fn ensure_writable(root: &Path, mode: StorageMode) -> Result<(), AppError> {
    fs::create_dir_all(root).map_err(|error| storage_error(root, mode, error))?;
    let probe = root.join(format!(".sorttrack-write-test-{}", Uuid::new_v4()));
    let result = (|| -> Result<(), std::io::Error> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&probe)?;
        file.write_all(b"SortTrack storage probe")?;
        file.sync_all()?;
        drop(file);
        fs::remove_file(&probe)?;
        Ok(())
    })();
    if let Err(error) = result {
        let _ = fs::remove_file(&probe);
        return Err(storage_error(root, mode, error));
    }
    Ok(())
}

fn storage_error(root: &Path, mode: StorageMode, error: std::io::Error) -> AppError {
    let label = match mode {
        StorageMode::Installed => "системная папка данных",
        StorageMode::Portable => "папка data portable-версии",
    };
    AppError::Task(format!(
        "SortTrack не может записывать в {label} {}: {error}",
        root.display()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temporary_directory(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!("sorttrack-{label}-{}", Uuid::new_v4()))
    }

    #[test]
    fn installed_and_portable_profiles_never_share_a_data_root() {
        let root = temporary_directory("profile-isolation");
        let installed = root.join("installed-data");
        let executable_directory = root.join("portable");
        fs::create_dir_all(&executable_directory).unwrap();
        let executable = executable_directory.join("SortTrack.exe");

        let installed_profile = resolve(&installed, &executable).unwrap();
        assert_eq!(installed_profile.mode, StorageMode::Installed);
        assert_eq!(
            installed_profile.data_root,
            installed.canonicalize().unwrap()
        );

        fs::write(executable_directory.join(PORTABLE_MARKER), b"portable\n").unwrap();
        let portable_profile = resolve(&installed, &executable).unwrap();
        assert_eq!(portable_profile.mode, StorageMode::Portable);
        assert_eq!(
            portable_profile.data_root,
            executable_directory.join("data").canonicalize().unwrap()
        );
        assert_ne!(installed_profile.data_root, portable_profile.data_root);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_a_non_file_portable_marker() {
        let root = temporary_directory("invalid-profile-marker");
        let executable_directory = root.join("portable");
        fs::create_dir_all(executable_directory.join(PORTABLE_MARKER)).unwrap();
        let error = resolve(
            &root.join("installed"),
            &executable_directory.join("SortTrack.exe"),
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("недопустимый тип"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn preserves_an_existing_legacy_installed_database() {
        let root = temporary_directory("legacy-profile");
        let preferred = root.join("SortTrack");
        let legacy = root.join("app.sorttrack.desktop");
        fs::create_dir_all(legacy.join("database")).unwrap();
        fs::write(legacy.join("database").join("sorttrack.db"), b"legacy").unwrap();
        assert_eq!(
            select_installed_data_root(&preferred, &legacy).unwrap(),
            legacy
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn refuses_to_choose_between_two_installed_databases() {
        let root = temporary_directory("profile-conflict");
        let preferred = root.join("SortTrack");
        let legacy = root.join("app.sorttrack.desktop");
        for directory in [&preferred, &legacy] {
            fs::create_dir_all(directory.join("database")).unwrap();
            fs::write(directory.join("database").join("sorttrack.db"), b"database").unwrap();
        }
        let error = select_installed_data_root(&preferred, &legacy)
            .unwrap_err()
            .to_string();
        assert!(error.contains("две базы"));
        fs::remove_dir_all(root).unwrap();
    }
}

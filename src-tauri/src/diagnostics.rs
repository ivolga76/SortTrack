use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use chrono::Utc;
use serde::Serialize;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{
    error::AppError,
    runtime_profile::{RuntimeProfile, StorageMode},
};

const LOG_PREFIX: &str = "sorttrack";
const MAX_LOG_BYTES: u64 = 2 * 1024 * 1024;
const MAX_LOG_FILES: usize = 5;
const MAX_LOG_AGE: Duration = Duration::from_secs(30 * 24 * 60 * 60);
const MAX_EXPORT_BYTES: u64 = 20 * 1024 * 1024;

#[derive(Clone)]
pub struct DiagnosticsState {
    inner: Arc<DiagnosticsInner>,
}

struct DiagnosticsInner {
    data_root: PathBuf,
    log_directory: PathBuf,
    crash_directory: PathBuf,
    mode: StorageMode,
    write_lock: Mutex<()>,
    max_log_bytes: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticsPreview {
    pub expected_state: String,
    pub log_files: usize,
    pub crash_reports: usize,
    pub total_bytes: u64,
    pub redact_paths: bool,
    pub included_fields: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticsExportReport {
    pub path: String,
    pub size_bytes: u64,
    pub sha256: String,
    pub log_files: usize,
    pub crash_reports: usize,
    pub paths_redacted: bool,
}

#[derive(Serialize)]
struct LogRecord<'a> {
    timestamp: String,
    level: &'a str,
    event: &'a str,
    message: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DiagnosticBundle {
    schema: &'static str,
    created_at: String,
    application_version: &'static str,
    platform: &'static str,
    architecture: &'static str,
    storage_mode: &'static str,
    paths_redacted: bool,
    logs: Vec<BundleFile>,
    crash_reports: Vec<BundleFile>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BundleFile {
    name: String,
    content: String,
}

#[derive(Clone)]
struct SnapshotFile {
    category: &'static str,
    name: String,
    path: PathBuf,
    size: u64,
    modified_ms: u128,
}

impl DiagnosticsState {
    pub fn new(profile: &RuntimeProfile) -> Result<Self, AppError> {
        Self::new_with_limit(profile, MAX_LOG_BYTES)
    }

    fn new_with_limit(profile: &RuntimeProfile, max_log_bytes: u64) -> Result<Self, AppError> {
        let log_directory = profile.data_root.join("logs");
        let crash_directory = profile.data_root.join("crashes");
        fs::create_dir_all(&log_directory)?;
        fs::create_dir_all(&crash_directory)?;
        let state = Self {
            inner: Arc::new(DiagnosticsInner {
                data_root: profile.data_root.clone(),
                log_directory,
                crash_directory,
                mode: profile.mode,
                write_lock: Mutex::new(()),
                max_log_bytes,
            }),
        };
        state.maintain_logs()?;
        Ok(state)
    }

    pub fn info(&self, event: &str, message: &str) {
        let _ = self.write_record("info", event, message);
    }

    pub fn warn(&self, event: &str, message: &str) {
        let _ = self.write_record("warn", event, message);
    }

    fn write_record(&self, level: &str, event: &str, message: &str) -> Result<(), AppError> {
        let _guard = self
            .inner
            .write_lock
            .lock()
            .map_err(|_| AppError::StateUnavailable)?;
        self.rotate_if_needed()?;
        let line = serde_json::to_string(&LogRecord {
            timestamp: Utc::now().to_rfc3339(),
            level,
            event,
            message,
        })?;
        let path = self.active_log_path();
        let mut file = OpenOptions::new().create(true).append(true).open(path)?;
        file.write_all(line.as_bytes())?;
        file.write_all(b"\n")?;
        file.flush()?;
        Ok(())
    }

    pub fn install_panic_hook(&self) {
        let crash_directory = self.inner.crash_directory.clone();
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |panic| {
            let message = panic
                .payload()
                .downcast_ref::<&str>()
                .copied()
                .or_else(|| panic.payload().downcast_ref::<String>().map(String::as_str))
                .unwrap_or("panic without string payload");
            let location = panic
                .location()
                .map(|item| format!("{}:{}:{}", item.file(), item.line(), item.column()));
            let report = serde_json::json!({
                "schema": "sorttrack-crash-1",
                "timestamp": Utc::now().to_rfc3339(),
                "message": message,
                "location": location,
                "applicationVersion": env!("CARGO_PKG_VERSION"),
            });
            let path = crash_directory.join(format!(
                "crash-{}-{}.json",
                Utc::now().format("%Y%m%dT%H%M%SZ"),
                Uuid::new_v4()
            ));
            if let Ok(mut file) = OpenOptions::new().create_new(true).write(true).open(path) {
                let _ = file.write_all(format!("{}\n", report).as_bytes());
                let _ = file.sync_all();
            }
            previous(panic);
        }));
    }

    pub fn preview(&self, redact_paths: bool) -> Result<DiagnosticsPreview, AppError> {
        let snapshot = self.snapshot()?;
        let total_bytes = snapshot.iter().map(|item| item.size).sum();
        Ok(DiagnosticsPreview {
            expected_state: snapshot_state(&snapshot),
            log_files: snapshot
                .iter()
                .filter(|item| item.category == "log")
                .count(),
            crash_reports: snapshot
                .iter()
                .filter(|item| item.category == "crash")
                .count(),
            total_bytes,
            redact_paths,
            included_fields: vec![
                "версия SortTrack, Windows и архитектура".into(),
                "локальные JSONL-логи приложения".into(),
                "локальные crash reports, если они есть".into(),
            ],
            warnings: vec![
                "MP3/FLAC, база SQLite, плейлисты и музыкальные метаданные не включаются".into(),
                "Пакет никуда не отправляется автоматически".into(),
            ],
        })
    }

    pub fn export(
        &self,
        destination: &Path,
        expected_state: &str,
        redact_paths: bool,
    ) -> Result<DiagnosticsExportReport, AppError> {
        let snapshot = self.snapshot()?;
        if snapshot_state(&snapshot) != expected_state {
            return Err(AppError::Task(
                "Логи изменились после предварительного просмотра; выполните preview повторно"
                    .into(),
            ));
        }
        let total: u64 = snapshot.iter().map(|item| item.size).sum();
        if total > MAX_EXPORT_BYTES {
            return Err(AppError::Task(
                "Диагностический пакет превышает безопасный лимит 20 МБ".into(),
            ));
        }
        if destination.exists() {
            return Err(AppError::Task(
                "Диагностический файл уже существует; перезапись запрещена".into(),
            ));
        }
        let parent = destination.parent().ok_or_else(|| {
            AppError::Task("Не удалось определить папку экспорта диагностики".into())
        })?;
        fs::create_dir_all(parent)?;
        let mut logs = Vec::new();
        let mut crashes = Vec::new();
        for item in &snapshot {
            let mut content = fs::read_to_string(&item.path)?;
            if redact_paths {
                content = redact_local_paths(&content, &self.inner.data_root);
            }
            let target = if item.category == "log" {
                &mut logs
            } else {
                &mut crashes
            };
            target.push(BundleFile {
                name: item.name.clone(),
                content,
            });
        }
        let bundle = DiagnosticBundle {
            schema: "sorttrack-diagnostics-1",
            created_at: Utc::now().to_rfc3339(),
            application_version: env!("CARGO_PKG_VERSION"),
            platform: std::env::consts::OS,
            architecture: std::env::consts::ARCH,
            storage_mode: match self.inner.mode {
                StorageMode::Installed => "installed",
                StorageMode::Portable => "portable",
            },
            paths_redacted: redact_paths,
            logs,
            crash_reports: crashes,
        };
        let bytes = serde_json::to_vec_pretty(&bundle)?;
        let temporary = parent.join(format!(".sorttrack-diagnostics-{}.tmp", Uuid::new_v4()));
        let result = (|| -> Result<(), AppError> {
            let mut file = OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&temporary)?;
            file.write_all(&bytes)?;
            file.write_all(b"\n")?;
            file.sync_all()?;
            fs::hard_link(&temporary, destination).map_err(|error| {
                AppError::Task(format!(
                    "Не удалось опубликовать диагностический пакет без перезаписи: {error}"
                ))
            })?;
            Ok(())
        })();
        let _ = fs::remove_file(&temporary);
        result?;
        let size_bytes = destination.metadata()?.len();
        let sha256 = hex_sha256(&fs::read(destination)?);
        self.info(
            "diagnostics_exported",
            "Пользователь экспортировал локальный диагностический пакет",
        );
        Ok(DiagnosticsExportReport {
            path: destination.to_string_lossy().into_owned(),
            size_bytes,
            sha256,
            log_files: snapshot
                .iter()
                .filter(|item| item.category == "log")
                .count(),
            crash_reports: snapshot
                .iter()
                .filter(|item| item.category == "crash")
                .count(),
            paths_redacted: redact_paths,
        })
    }

    pub fn open_log_directory(&self) -> Result<(), AppError> {
        #[cfg(windows)]
        {
            std::process::Command::new("explorer.exe")
                .arg(&self.inner.log_directory)
                .spawn()?;
            Ok(())
        }
        #[cfg(not(windows))]
        Err(AppError::Task(
            "Открытие папки логов поддерживается только в Windows-сборке".into(),
        ))
    }

    pub fn clear(&self) -> Result<u64, AppError> {
        let _guard = self
            .inner
            .write_lock
            .lock()
            .map_err(|_| AppError::StateUnavailable)?;
        let snapshot = self.snapshot_unlocked()?;
        let mut removed = 0;
        for item in snapshot {
            fs::remove_file(item.path)?;
            removed += 1;
        }
        Ok(removed)
    }

    fn maintain_logs(&self) -> Result<(), AppError> {
        let now = SystemTime::now();
        let mut logs = self.snapshot_category(&self.inner.log_directory, "log", "jsonl")?;
        for item in &logs {
            let modified =
                UNIX_EPOCH + Duration::from_millis(item.modified_ms.min(u64::MAX as u128) as u64);
            if now.duration_since(modified).unwrap_or_default() > MAX_LOG_AGE {
                let _ = fs::remove_file(&item.path);
            }
        }
        logs = self.snapshot_category(&self.inner.log_directory, "log", "jsonl")?;
        logs.sort_by_key(|item| std::cmp::Reverse(item.modified_ms));
        // Reserve one slot for the active file created after a rotation.
        for item in logs.into_iter().skip(MAX_LOG_FILES.saturating_sub(1)) {
            let _ = fs::remove_file(item.path);
        }
        Ok(())
    }

    fn rotate_if_needed(&self) -> Result<(), AppError> {
        let active = self.active_log_path();
        if active.try_exists()? && active.metadata()?.len() >= self.inner.max_log_bytes {
            let rotated = self.inner.log_directory.join(format!(
                "{LOG_PREFIX}-{}-{}.jsonl",
                Utc::now().format("%Y%m%dT%H%M%SZ"),
                Uuid::new_v4()
            ));
            fs::rename(active, rotated)?;
            self.maintain_logs()?;
        }
        Ok(())
    }

    fn active_log_path(&self) -> PathBuf {
        self.inner.log_directory.join(format!("{LOG_PREFIX}.jsonl"))
    }

    fn snapshot(&self) -> Result<Vec<SnapshotFile>, AppError> {
        let _guard = self
            .inner
            .write_lock
            .lock()
            .map_err(|_| AppError::StateUnavailable)?;
        self.snapshot_unlocked()
    }

    fn snapshot_unlocked(&self) -> Result<Vec<SnapshotFile>, AppError> {
        let mut files = self.snapshot_category(&self.inner.log_directory, "log", "jsonl")?;
        files.extend(self.snapshot_category(&self.inner.crash_directory, "crash", "json")?);
        files
            .sort_by(|left, right| (left.category, &left.name).cmp(&(right.category, &right.name)));
        Ok(files)
    }

    fn snapshot_category(
        &self,
        directory: &Path,
        category: &'static str,
        extension: &str,
    ) -> Result<Vec<SnapshotFile>, AppError> {
        let mut files = Vec::new();
        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            let file_type = entry.file_type()?;
            if file_type.is_symlink() || !file_type.is_file() {
                continue;
            }
            let metadata = entry.metadata()?;
            if entry.path().extension().and_then(|item| item.to_str()) != Some(extension) {
                continue;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            if category == "log" && !name.starts_with(LOG_PREFIX) {
                continue;
            }
            if category == "crash" && !name.starts_with("crash-") {
                continue;
            }
            let modified_ms = metadata
                .modified()?
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis();
            files.push(SnapshotFile {
                category,
                name,
                path: entry.path(),
                size: metadata.len(),
                modified_ms,
            });
        }
        Ok(files)
    }
}

fn snapshot_state(files: &[SnapshotFile]) -> String {
    let mut hash = Sha256::new();
    for file in files {
        hash.update(file.category.as_bytes());
        hash.update([0]);
        hash.update(file.name.as_bytes());
        hash.update(file.size.to_le_bytes());
        hash.update(file.modified_ms.to_le_bytes());
    }
    hash.finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn hex_sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn redact_local_paths(text: &str, data_root: &Path) -> String {
    let escaped_root = data_root.to_string_lossy().replace('\\', "\\\\");
    let value = text.replace(&escaped_root, "%SORTTRACK_DATA%");
    let chars: Vec<char> = value.chars().collect();
    let mut output = String::with_capacity(value.len());
    let mut index = 0;
    while index < chars.len() {
        let drive_path = index + 2 < chars.len()
            && chars[index].is_ascii_alphabetic()
            && chars[index + 1] == ':'
            && (chars[index + 2] == '\\' || chars[index + 2] == '/');
        let unc_path = index + 1 < chars.len() && chars[index] == '\\' && chars[index + 1] == '\\';
        if drive_path || unc_path {
            output.push_str("[LOCAL_PATH]");
            index += if drive_path { 3 } else { 2 };
            while index < chars.len()
                && !chars[index].is_whitespace()
                && !matches!(chars[index], '"' | '\'' | ',' | '}' | ']')
            {
                index += 1;
            }
        } else {
            output.push(chars[index]);
            index += 1;
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_state(label: &str, limit: u64) -> (DiagnosticsState, PathBuf) {
        let root =
            std::env::temp_dir().join(format!("sorttrack-diagnostics-{label}-{}", Uuid::new_v4()));
        let profile = RuntimeProfile {
            mode: StorageMode::Portable,
            data_root: root.clone(),
        };
        (
            DiagnosticsState::new_with_limit(&profile, limit).unwrap(),
            root,
        )
    }

    #[test]
    fn rotates_logs_and_builds_a_stable_preview() {
        let (state, root) = test_state("rotation", 80);
        for index in 0..12 {
            state.info(
                "test_event",
                &format!("record {index} with enough text to rotate"),
            );
        }
        let preview = state.preview(true).unwrap();
        assert!(preview.log_files >= 2 && preview.log_files <= MAX_LOG_FILES);
        assert_eq!(
            preview.expected_state,
            state.preview(true).unwrap().expected_state
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn export_redacts_paths_rejects_overwrite_and_detects_changed_logs() {
        let (state, root) = test_state("export", 1024 * 1024);
        state.warn(
            "file_error",
            r#"Не удалось открыть C:\Music\Private\track.flac"#,
        );
        let preview = state.preview(true).unwrap();
        let destination = root.join("diagnostics.json");
        let report = state
            .export(&destination, &preview.expected_state, true)
            .unwrap();
        assert!(report.paths_redacted);
        let content = fs::read_to_string(&destination).unwrap();
        assert!(content.contains("[LOCAL_PATH]"));
        assert!(!content.contains("Private"));
        assert!(
            state
                .export(
                    &destination,
                    &state.preview(true).unwrap().expected_state,
                    true
                )
                .is_err()
        );
        let stale = state.preview(true).unwrap();
        state.info("after_preview", "changed");
        assert!(
            state
                .export(&root.join("second.json"), &stale.expected_state, true)
                .is_err()
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn clear_removes_only_owned_diagnostic_files() {
        let (state, root) = test_state("clear", 1024 * 1024);
        state.info("test", "record");
        fs::write(root.join("logs").join("keep.txt"), b"keep").unwrap();
        fs::write(root.join("crashes").join("keep.json"), b"keep").unwrap();
        assert_eq!(state.clear().unwrap(), 1);
        assert!(root.join("logs").join("keep.txt").is_file());
        assert!(root.join("crashes").join("keep.json").is_file());
        fs::remove_dir_all(root).unwrap();
    }
}

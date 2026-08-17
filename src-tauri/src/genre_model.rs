use std::{
    collections::{BTreeMap, BTreeSet},
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    audio_features::{AudioFeatureVector, EXTRACTOR_VERSION, extract_file},
    error::AppError,
    file_copy::sha256_file,
    file_identity::file_identity,
    models::{ConfusionCell, GenreClassMetric, GenreModelEvaluation},
    repository::AppState,
};

const MODEL_VERSION: &str = "nearest-centroid-baseline-1";
const RELEASE_ACCURACY: f64 = 0.70;
const REQUIRED_ENTRIES: usize = 1000;
const REQUIRED_CLASSES: usize = 8;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Manifest {
    taxonomy_version: u32,
    entries: Vec<ManifestEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ManifestEntry {
    path: String,
    size_bytes: u64,
    modified_at_ms: i64,
    genre: String,
    split: String,
}

#[derive(Debug)]
struct FeatureFile {
    id: String,
    path: PathBuf,
    root: Option<PathBuf>,
    size_bytes: u64,
    modified_at_ms: i64,
    identity: Option<String>,
}

#[derive(Debug, Clone)]
struct Example {
    genre: String,
    split: String,
    values: Vec<f64>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct CentroidModel {
    model_version: String,
    feature_version: String,
    feature_mean: Vec<f64>,
    feature_scale: Vec<f64>,
    centroids: BTreeMap<String, Vec<f64>>,
    calibration_distance_scale: f64,
}

#[derive(Debug, Clone)]
pub(crate) struct ApprovedGenrePrediction {
    pub genre: String,
    pub score: f64,
    pub model_version: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ModelRunReport {
    schema_version: u32,
    exported_at: String,
    run_id: String,
    model_version: String,
    feature_version: String,
    taxonomy_version: u32,
    top1_accuracy: f64,
    meets_release_threshold: bool,
    dataset_manifest_sha256: Option<String>,
    created_at: String,
    metrics: serde_json::Value,
}

pub fn latest_evaluation(state: &AppState) -> Result<Option<GenreModelEvaluation>, AppError> {
    let metrics = state
        .connection()?
        .query_row(
            "SELECT metrics_json FROM genre_model_runs ORDER BY created_at DESC, id DESC LIMIT 1",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    metrics
        .map(|json| {
            serde_json::from_str(&json)
                .map_err(|error| AppError::Genre(format!("Некорректный отчёт model run: {error}")))
        })
        .transpose()
}

pub fn export_latest_report(state: &AppState, destination: &Path) -> Result<String, AppError> {
    if destination.extension().and_then(|value| value.to_str()) != Some("json") {
        return Err(AppError::Genre(
            "Отчёт модели должен иметь расширение .json".into(),
        ));
    }
    if destination.exists() {
        return Err(AppError::Genre(
            "Файл отчёта уже существует; выберите другое имя".into(),
        ));
    }
    let report = state
        .connection()?
        .query_row(
            "SELECT r.id, r.model_version, r.feature_version, r.taxonomy_version, r.top1_accuracy, r.meets_release_threshold, d.manifest_sha256, r.created_at, r.metrics_json FROM genre_model_runs r LEFT JOIN genre_dataset_exports d ON d.id = r.dataset_export_id ORDER BY r.created_at DESC, r.id DESC LIMIT 1",
            [],
            |row| {
                let metrics_json: String = row.get(8)?;
                Ok(ModelRunReport {
                    schema_version: 1,
                    exported_at: chrono::Utc::now().to_rfc3339(),
                    run_id: row.get(0)?,
                    model_version: row.get(1)?,
                    feature_version: row.get(2)?,
                    taxonomy_version: row.get(3)?,
                    top1_accuracy: row.get(4)?,
                    meets_release_threshold: row.get(5)?,
                    dataset_manifest_sha256: row.get(6)?,
                    created_at: row.get(7)?,
                    metrics: serde_json::from_str(&metrics_json).unwrap_or(serde_json::Value::Null),
                })
            },
        )
        .optional()?
        .ok_or_else(|| AppError::Genre("Сначала выполните оценку модели".into()))?;
    if report.metrics.is_null() {
        return Err(AppError::Genre(
            "Сохранённые метрики модели повреждены".into(),
        ));
    }
    let bytes =
        serde_json::to_vec_pretty(&report).map_err(|error| AppError::Genre(error.to_string()))?;
    let parent = destination
        .parent()
        .ok_or_else(|| AppError::Genre("Не удалось определить папку отчёта".into()))?;
    let temporary = parent.join(format!(".sorttrack-model-report-{}.tmp", Uuid::new_v4()));
    let result = (|| -> Result<(), AppError> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        std::fs::hard_link(&temporary, destination).map_err(|error| {
            if destination.exists() {
                AppError::Genre("Файл отчёта уже существует; выберите другое имя".into())
            } else {
                AppError::Io(error)
            }
        })?;
        std::fs::remove_file(&temporary)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result?;
    Ok(destination.to_string_lossy().into_owned())
}

pub(crate) fn approved_predictions(
    state: &AppState,
    track_id: &str,
) -> Result<Option<Vec<ApprovedGenrePrediction>>, AppError> {
    let approved = state
        .connection()?
        .query_row(
            "SELECT model_version, feature_version, model_json FROM genre_model_runs WHERE meets_release_threshold = 1 ORDER BY created_at DESC, id DESC LIMIT 1",
            [],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?)),
        )
        .optional()?;
    let Some((model_version, feature_version, model_json)) = approved else {
        return Ok(None);
    };
    if feature_version != EXTRACTOR_VERSION {
        return Ok(None);
    }
    let model: CentroidModel = serde_json::from_str(&model_json)
        .map_err(|error| AppError::Genre(format!("Некорректная одобренная модель: {error}")))?;
    if model.feature_version != EXTRACTOR_VERSION || model.model_version != model_version {
        return Ok(None);
    }
    let file = load_track_file(state, track_id)?;
    let vector = load_or_extract_file(state, &file)?;
    let scale = model.calibration_distance_scale.max(1e-6);
    let predictions = predict(&model, &vector.values)
        .into_iter()
        .take(3)
        .map(|(genre, distance)| ApprovedGenrePrediction {
            genre,
            score: (-distance / scale).exp().clamp(0.0, 1.0),
            model_version: model_version.clone(),
        })
        .collect();
    Ok(Some(predictions))
}

pub fn evaluate_latest(state: &AppState) -> Result<GenreModelEvaluation, AppError> {
    let (dataset_export_id, manifest_path, expected_manifest_sha): (String, String, String) = state
        .connection()?
        .query_row(
            "SELECT id, destination_path, manifest_sha256 FROM genre_dataset_exports ORDER BY created_at DESC, id DESC LIMIT 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?
        .ok_or_else(|| AppError::Genre("Сначала создайте validation dataset manifest".into()))?;
    let bytes = std::fs::read(&manifest_path)?;
    let actual_manifest_sha = sha256_file(Path::new(&manifest_path))?;
    if actual_manifest_sha != expected_manifest_sha {
        return Err(AppError::Genre(
            "Manifest изменился после экспорта; обучение остановлено".into(),
        ));
    }
    let manifest: Manifest = serde_json::from_slice(&bytes)
        .map_err(|error| AppError::Genre(format!("Некорректный manifest: {error}")))?;
    if manifest.taxonomy_version != 1 {
        return Err(AppError::Genre(
            "Версия таксономии manifest не поддерживается".into(),
        ));
    }
    let train_entries = manifest
        .entries
        .iter()
        .filter(|entry| entry.split == "train")
        .count() as u64;
    let calibration_entries = manifest
        .entries
        .iter()
        .filter(|entry| entry.split == "calibration")
        .count() as u64;
    let test_entries = manifest
        .entries
        .iter()
        .filter(|entry| entry.split == "test")
        .count() as u64;
    if train_entries == 0 || test_entries == 0 {
        return Err(AppError::Genre(
            "Manifest должен содержать train и test".into(),
        ));
    }

    let mut examples = Vec::new();
    let mut failed_features = 0u64;
    let mut failures_by_test_genre = BTreeMap::<String, u64>::new();
    for entry in &manifest.entries {
        match load_or_extract(state, entry) {
            Ok(vector) => examples.push(Example {
                genre: entry.genre.clone(),
                split: entry.split.clone(),
                values: vector.values,
            }),
            Err(_) => {
                failed_features += 1;
                if entry.split == "test" {
                    *failures_by_test_genre
                        .entry(entry.genre.clone())
                        .or_default() += 1;
                }
            }
        }
    }
    let train = examples
        .iter()
        .filter(|example| example.split == "train")
        .cloned()
        .collect::<Vec<_>>();
    let calibration = examples
        .iter()
        .filter(|example| example.split == "calibration")
        .cloned()
        .collect::<Vec<_>>();
    let test = examples
        .iter()
        .filter(|example| example.split == "test")
        .cloned()
        .collect::<Vec<_>>();
    let mut model = fit(&train)?;
    model.calibration_distance_scale = calibration_scale(&model, &calibration);

    let mut confusion = BTreeMap::<(String, String), u64>::new();
    let mut correct = 0u64;
    let mut support = failures_by_test_genre;
    let mut correct_by_genre = BTreeMap::<String, u64>::new();
    for example in &test {
        *support.entry(example.genre.clone()).or_default() += 1;
        let predicted = predict(&model, &example.values)
            .first()
            .map(|(genre, _)| genre.clone())
            .ok_or_else(|| AppError::Genre("Модель не содержит классов".into()))?;
        *confusion
            .entry((example.genre.clone(), predicted.clone()))
            .or_default() += 1;
        if predicted == example.genre {
            correct += 1;
            *correct_by_genre.entry(example.genre.clone()).or_default() += 1;
        }
    }
    let top1_accuracy = correct as f64 / test_entries.max(1) as f64;
    let class_metrics = support
        .into_iter()
        .map(|(genre, support)| {
            let correct = correct_by_genre.get(&genre).copied().unwrap_or(0);
            GenreClassMetric {
                genre,
                support,
                correct,
                recall: correct as f64 / support.max(1) as f64,
            }
        })
        .collect::<Vec<_>>();
    let confusion = confusion
        .into_iter()
        .map(|((actual, predicted), count)| ConfusionCell {
            actual,
            predicted,
            count,
        })
        .collect::<Vec<_>>();
    let present_classes = manifest
        .entries
        .iter()
        .map(|entry| entry.genre.as_str())
        .collect::<BTreeSet<_>>()
        .len();
    let meets_release_threshold = top1_accuracy >= RELEASE_ACCURACY
        && manifest.entries.len() >= REQUIRED_ENTRIES
        && present_classes >= REQUIRED_CLASSES
        && failed_features == 0;
    let mut warnings = Vec::new();
    if manifest.entries.len() < REQUIRED_ENTRIES {
        warnings.push(format!(
            "Manifest содержит {} записей; критерий выпуска требует не менее {REQUIRED_ENTRIES}",
            manifest.entries.len()
        ));
    }
    if present_classes < REQUIRED_CLASSES {
        warnings.push(format!(
            "В manifest представлены {present_classes} из {REQUIRED_CLASSES} широких жанров"
        ));
    }
    if failed_features > 0 {
        warnings.push(format!("Не удалось извлечь признаки для {failed_features} файлов; они считаются ошибками оценки"));
    }
    if top1_accuracy < RELEASE_ACCURACY {
        warnings.push(format!("Top-1 accuracy {:.1}% ниже порога 70%; baseline нельзя использовать как готовую модель", top1_accuracy * 100.0));
    }

    let run_id = Uuid::new_v4().to_string();
    let evaluation = GenreModelEvaluation {
        run_id: run_id.clone(),
        model_version: MODEL_VERSION.into(),
        feature_version: EXTRACTOR_VERSION.into(),
        dataset_manifest_sha256: actual_manifest_sha,
        top1_accuracy,
        meets_release_threshold,
        train_entries,
        calibration_entries,
        test_entries,
        failed_features,
        class_metrics,
        confusion,
        warnings,
    };
    let connection = state.connection()?;
    connection.execute(
        "INSERT INTO genre_model_runs(id, dataset_export_id, model_version, feature_version, taxonomy_version, top1_accuracy, meets_release_threshold, model_json, metrics_json) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![run_id, dataset_export_id, MODEL_VERSION, EXTRACTOR_VERSION, manifest.taxonomy_version, evaluation.top1_accuracy, evaluation.meets_release_threshold, serde_json::to_string(&model).map_err(|error| AppError::Genre(error.to_string()))?, serde_json::to_string(&evaluation).map_err(|error| AppError::Genre(error.to_string()))?],
    )?;
    Ok(evaluation)
}

fn load_or_extract(
    state: &AppState,
    entry: &ManifestEntry,
) -> Result<AudioFeatureVector, AppError> {
    let file = load_file(state, entry)?;
    load_or_extract_file(state, &file)
}

fn load_or_extract_file(
    state: &AppState,
    file: &FeatureFile,
) -> Result<AudioFeatureVector, AppError> {
    validate_file(file)?;
    if let Some(json) = state.connection()?.query_row(
        "SELECT features_json FROM audio_feature_cache WHERE track_file_id = ?1 AND extractor_version = ?2 AND size_bytes = ?3 AND modified_at_ms = ?4 AND coalesce(file_identity, '') = coalesce(?5, '')",
        params![file.id, EXTRACTOR_VERSION, file.size_bytes, file.modified_at_ms, file.identity],
        |row| row.get::<_, String>(0),
    ).optional()? {
        return serde_json::from_str(&json).map_err(|error| AppError::Genre(error.to_string()));
    }
    let vector = extract_file(&file.path).map_err(AppError::Genre)?;
    validate_file(file)?;
    let connection = state.connection()?;
    connection.execute(
        "INSERT INTO audio_feature_cache(track_file_id, extractor_version, features_json, file_identity, size_bytes, modified_at_ms) VALUES (?1, ?2, ?3, ?4, ?5, ?6) ON CONFLICT(track_file_id) DO UPDATE SET extractor_version=excluded.extractor_version, features_json=excluded.features_json, file_identity=excluded.file_identity, size_bytes=excluded.size_bytes, modified_at_ms=excluded.modified_at_ms, extracted_at=CURRENT_TIMESTAMP",
        params![file.id, EXTRACTOR_VERSION, serde_json::to_string(&vector).map_err(|error| AppError::Genre(error.to_string()))?, file.identity, file.size_bytes, file.modified_at_ms],
    )?;
    Ok(vector)
}

fn load_track_file(state: &AppState, track_id: &str) -> Result<FeatureFile, AppError> {
    state
        .connection()?
        .query_row(
            "SELECT f.id, f.canonical_path, r.canonical_path, f.size_bytes, f.modified_at_ms, f.file_identity FROM track_files f LEFT JOIN library_roots r ON r.id = f.library_root_id WHERE f.track_id = ?1 AND f.availability_status = 'available' ORDER BY CASE lower(f.extension) WHEN 'flac' THEN 0 ELSE 1 END, f.updated_at DESC LIMIT 1",
            [track_id],
            |row| Ok(FeatureFile {
                id: row.get(0)?,
                path: PathBuf::from(row.get::<_, String>(1)?),
                root: row.get::<_, Option<String>>(2)?.map(PathBuf::from),
                size_bytes: row.get(3)?,
                modified_at_ms: row.get(4)?,
                identity: row.get(5)?,
            }),
        )
        .optional()?
        .ok_or_else(|| AppError::Genre(format!("Нет доступного аудиофайла для трека {track_id}")))
}

fn load_file(state: &AppState, entry: &ManifestEntry) -> Result<FeatureFile, AppError> {
    let file = state.connection()?.query_row(
        "SELECT f.id, f.canonical_path, r.canonical_path, f.size_bytes, f.modified_at_ms, f.file_identity FROM track_files f LEFT JOIN library_roots r ON r.id = f.library_root_id WHERE f.canonical_path = ?1 AND f.availability_status = 'available'",
        [&entry.path],
        |row| Ok(FeatureFile { id: row.get(0)?, path: PathBuf::from(row.get::<_, String>(1)?), root: row.get::<_, Option<String>>(2)?.map(PathBuf::from), size_bytes: row.get(3)?, modified_at_ms: row.get(4)?, identity: row.get(5)? }),
    ).optional()?.ok_or_else(|| AppError::Genre(format!("Файл manifest отсутствует в библиотеке: {}", entry.path)))?;
    if file.size_bytes != entry.size_bytes || file.modified_at_ms != entry.modified_at_ms {
        return Err(AppError::Genre(format!(
            "Файл изменился после создания manifest: {}",
            entry.path
        )));
    }
    Ok(file)
}

fn validate_file(file: &FeatureFile) -> Result<(), AppError> {
    let path = file.path.canonicalize()?;
    if let Some(root) = &file.root {
        if !path.starts_with(root.canonicalize()?) {
            return Err(AppError::Genre(
                "Файл находится вне зарегистрированной библиотеки".into(),
            ));
        }
    }
    let metadata = path.metadata()?;
    let modified = metadata
        .modified()?
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;
    if metadata.len() != file.size_bytes || modified != file.modified_at_ms {
        return Err(AppError::Genre(
            "Файл изменился во время извлечения признаков".into(),
        ));
    }
    if file
        .identity
        .as_deref()
        .is_some_and(|expected| file_identity(&path).as_deref().ok() != Some(expected))
    {
        return Err(AppError::Genre(
            "Физическая идентичность файла изменилась".into(),
        ));
    }
    Ok(())
}

fn fit(train: &[Example]) -> Result<CentroidModel, AppError> {
    let dimensions = train
        .first()
        .map(|example| example.values.len())
        .ok_or_else(|| AppError::Genre("Train пуст после извлечения признаков".into()))?;
    if train
        .iter()
        .any(|example| example.values.len() != dimensions)
    {
        return Err(AppError::Genre(
            "Размерность аудиопризнаков различается".into(),
        ));
    }
    let mut mean = vec![0.0; dimensions];
    for example in train {
        for (index, value) in example.values.iter().enumerate() {
            mean[index] += value;
        }
    }
    for value in &mut mean {
        *value /= train.len() as f64;
    }
    let mut scale = vec![0.0; dimensions];
    for example in train {
        for (index, value) in example.values.iter().enumerate() {
            scale[index] += (value - mean[index]).powi(2);
        }
    }
    for value in &mut scale {
        *value = (*value / train.len() as f64).sqrt().max(1e-6);
    }
    let mut sums = BTreeMap::<String, (Vec<f64>, usize)>::new();
    for example in train {
        let normalized = normalize(&example.values, &mean, &scale);
        let entry = sums
            .entry(example.genre.clone())
            .or_insert_with(|| (vec![0.0; dimensions], 0));
        for (target, value) in entry.0.iter_mut().zip(normalized) {
            *target += value;
        }
        entry.1 += 1;
    }
    if sums.len() < 2 {
        return Err(AppError::Genre(
            "Для baseline нужны минимум два жанра в train".into(),
        ));
    }
    let centroids = sums
        .into_iter()
        .map(|(genre, (mut values, count))| {
            for value in &mut values {
                *value /= count as f64;
            }
            (genre, values)
        })
        .collect();
    Ok(CentroidModel {
        model_version: MODEL_VERSION.into(),
        feature_version: EXTRACTOR_VERSION.into(),
        feature_mean: mean,
        feature_scale: scale,
        centroids,
        calibration_distance_scale: 1.0,
    })
}

fn predict(model: &CentroidModel, values: &[f64]) -> Vec<(String, f64)> {
    let normalized = normalize(values, &model.feature_mean, &model.feature_scale);
    let mut result = model
        .centroids
        .iter()
        .map(|(genre, centroid)| {
            let distance = normalized
                .iter()
                .zip(centroid)
                .map(|(left, right)| (left - right).powi(2))
                .sum::<f64>()
                / normalized.len().max(1) as f64;
            (genre.clone(), distance.sqrt())
        })
        .collect::<Vec<_>>();
    result.sort_by(|left, right| {
        left.1
            .total_cmp(&right.1)
            .then_with(|| left.0.cmp(&right.0))
    });
    result
}

fn calibration_scale(model: &CentroidModel, calibration: &[Example]) -> f64 {
    let mut distances = calibration
        .iter()
        .filter_map(|example| {
            predict(model, &example.values)
                .first()
                .map(|(_, distance)| *distance)
        })
        .collect::<Vec<_>>();
    if distances.is_empty() {
        return 1.0;
    }
    distances.sort_by(f64::total_cmp);
    distances[distances.len() / 2].max(1e-6)
}

fn normalize(values: &[f64], mean: &[f64], scale: &[f64]) -> Vec<f64> {
    values
        .iter()
        .zip(mean)
        .zip(scale)
        .map(|((value, mean), scale)| (value - mean) / scale)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn centroid_baseline_separates_simple_feature_clusters() {
        let example = |genre: &str, a: f64, b: f64| Example {
            genre: genre.into(),
            split: "train".into(),
            values: vec![a, b],
        };
        let train = vec![
            example("House", 0.0, 0.1),
            example("House", 0.2, 0.0),
            example("Techno", 10.0, 9.8),
            example("Techno", 9.7, 10.1),
        ];
        let model = fit(&train).expect("model");
        assert_eq!(predict(&model, &[0.1, 0.0])[0].0, "House");
        assert_eq!(predict(&model, &[9.9, 10.0])[0].0, "Techno");
    }

    #[test]
    fn inference_does_not_touch_audio_without_an_approved_model() {
        let state = AppState::in_memory().expect("database");
        let predictions = approved_predictions(&state, "missing-track").expect("gate query");
        assert!(predictions.is_none());
    }

    #[test]
    fn calibrated_score_decreases_as_distance_grows() {
        let model = CentroidModel {
            model_version: MODEL_VERSION.into(),
            feature_version: EXTRACTOR_VERSION.into(),
            feature_mean: vec![0.0],
            feature_scale: vec![1.0],
            centroids: BTreeMap::from([("Techno".into(), vec![0.0])]),
            calibration_distance_scale: 2.0,
        };
        let near = predict(&model, &[0.5])[0].1;
        let far = predict(&model, &[4.0])[0].1;
        assert!(
            (-near / model.calibration_distance_scale).exp()
                > (-far / model.calibration_distance_scale).exp()
        );
    }

    #[test]
    fn model_report_export_never_overwrites() {
        let state = AppState::in_memory().expect("database");
        let evaluation = GenreModelEvaluation {
            run_id: "run-report".into(),
            model_version: MODEL_VERSION.into(),
            feature_version: EXTRACTOR_VERSION.into(),
            dataset_manifest_sha256: "manifest-sha".into(),
            top1_accuracy: 0.75,
            meets_release_threshold: true,
            train_entries: 700,
            calibration_entries: 150,
            test_entries: 150,
            failed_features: 0,
            class_metrics: Vec::new(),
            confusion: Vec::new(),
            warnings: Vec::new(),
        };
        state.connection().expect("connection").execute(
            "INSERT INTO genre_model_runs(id, model_version, feature_version, taxonomy_version, top1_accuracy, meets_release_threshold, model_json, metrics_json) VALUES (?1, ?2, ?3, 1, ?4, 1, '{}', ?5)",
            params![evaluation.run_id, MODEL_VERSION, EXTRACTOR_VERSION, evaluation.top1_accuracy, serde_json::to_string(&evaluation).expect("metrics")],
        ).expect("model run");
        let directory =
            std::env::temp_dir().join(format!("sorttrack-report-test-{}", Uuid::new_v4()));
        std::fs::create_dir(&directory).expect("temp directory");
        let destination = directory.join("report.json");
        export_latest_report(&state, &destination).expect("first export");
        let first = std::fs::read(&destination).expect("report");
        assert!(export_latest_report(&state, &destination).is_err());
        assert_eq!(std::fs::read(&destination).expect("unchanged"), first);
        std::fs::remove_file(destination).expect("remove report");
        std::fs::remove_dir(directory).expect("remove directory");
    }
}

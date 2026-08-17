use std::collections::HashMap;

use rusqlite::{OptionalExtension, params};
use uuid::Uuid;

use crate::{
    error::AppError,
    genre_model::{ApprovedGenrePrediction, approved_predictions},
    models::{GenreCandidate, GenreSuggestion},
    repository::AppState,
};

const INFERENCE_VERSION: &str = "metadata-path-bpm-1";
const TAXONOMY_VERSION: u32 = 1;
const BROAD_GENRES: [&str; 8] = [
    "Breaks",
    "Drum & Bass",
    "House",
    "Techno",
    "Trance",
    "Hardcore",
    "Hardstyle",
    "Electropunk",
];
const TECHNO_SUBGENRES: [&str; 7] = [
    "Melodic Techno",
    "Hard Techno",
    "Percussive Techno",
    "Peak Time Techno",
    "Acid Techno",
    "Raw Techno",
    "Schranz",
];

#[derive(Debug)]
struct TrackSignals {
    track_id: String,
    title: String,
    artist: String,
    confirmed_genre: Option<String>,
    confirmed_subgenre: Option<String>,
    locked: bool,
    path: String,
    filename: String,
    embedded_genre: Option<String>,
    bpm: Option<f64>,
}

#[derive(Debug, Default)]
struct Aggregate {
    score: f64,
    signals: Vec<String>,
}

pub fn suggest(state: &AppState, track_ids: &[String]) -> Result<Vec<GenreSuggestion>, AppError> {
    if track_ids.is_empty() || track_ids.len() > 1000 {
        return Err(AppError::Genre(
            "выберите от 1 до 1000 треков для жанрового анализа".into(),
        ));
    }
    let mut result = Vec::with_capacity(track_ids.len());
    for track_id in track_ids {
        let signals = load_track(state, track_id)?;
        let audio_predictions = approved_predictions(state, track_id)?;
        let inference_version = audio_predictions
            .as_ref()
            .and_then(|items| items.first())
            .map(|prediction| format!("{INFERENCE_VERSION}+{}", prediction.model_version))
            .unwrap_or_else(|| INFERENCE_VERSION.into());
        let candidates = infer(&signals, audio_predictions.as_deref());
        persist(state, track_id, &candidates, &inference_version)?;
        result.push(GenreSuggestion {
            track_id: signals.track_id,
            title: signals.title,
            artist: signals.artist,
            confirmed_genre: signals.confirmed_genre,
            confirmed_subgenre: signals.confirmed_subgenre,
            locked: signals.locked,
            candidates,
            inference_version,
            taxonomy_version: TAXONOMY_VERSION,
        });
    }
    Ok(result)
}

pub fn confirm(
    state: &AppState,
    track_id: &str,
    genre: &str,
    subgenre: Option<&str>,
    locked: bool,
) -> Result<(), AppError> {
    validate(genre, subgenre)?;
    let mut connection = state.connection()?;
    let transaction = connection.transaction()?;
    let before: Option<(Option<String>, Option<String>, bool)> = transaction
        .query_row(
            "SELECT confirmed_genre, confirmed_subgenre, genre_locked FROM tracks WHERE id = ?1",
            [track_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?;
    let (before_genre, before_subgenre, before_locked) =
        before.ok_or_else(|| AppError::Genre("трек не найден".into()))?;
    let source = if before_locked {
        if before_genre.as_deref() == Some(genre)
            && before_subgenre.as_deref() == subgenre
            && !locked
        {
            "user_unlock"
        } else {
            return Err(AppError::Genre(
                "жанр зафиксирован; сначала снимите фиксацию".into(),
            ));
        }
    } else {
        "user_confirmation"
    };
    transaction.execute(
        "UPDATE tracks SET confirmed_genre = ?1, confirmed_subgenre = ?2, genre_locked = ?3, updated_at = CURRENT_TIMESTAMP WHERE id = ?4",
        params![genre, subgenre, locked, track_id],
    )?;
    transaction.execute(
        "INSERT INTO genre_confirmation_history(id, track_id, before_genre, before_subgenre, before_locked, after_genre, after_subgenre, after_locked, source) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![Uuid::new_v4().to_string(), track_id, before_genre, before_subgenre, before_locked, genre, subgenre, locked, source],
    )?;
    transaction.commit()?;
    Ok(())
}

fn load_track(state: &AppState, track_id: &str) -> Result<TrackSignals, AppError> {
    let connection = state.connection()?;
    connection
        .query_row(
            "SELECT t.id, coalesce(m.title, t.preferred_title, f.filename), coalesce(m.artist, t.preferred_artist, ''), t.confirmed_genre, t.confirmed_subgenre, t.genre_locked, f.canonical_path, f.filename, m.original_genre_tag, coalesce(m.bpm, a.bpm) FROM tracks t JOIN track_files f ON f.id = (SELECT preferred.id FROM track_files preferred WHERE preferred.track_id = t.id ORDER BY CASE preferred.availability_status WHEN 'available' THEN 0 ELSE 1 END, CASE lower(preferred.extension) WHEN 'flac' THEN 0 ELSE 1 END LIMIT 1) LEFT JOIN track_metadata m ON m.track_file_id = f.id LEFT JOIN track_audio_analysis a ON a.track_file_id = f.id WHERE t.id = ?1",
            [track_id],
            |row| Ok(TrackSignals {
                track_id: row.get(0)?,
                title: row.get(1)?,
                artist: row.get(2)?,
                confirmed_genre: row.get(3)?,
                confirmed_subgenre: row.get(4)?,
                locked: row.get(5)?,
                path: row.get(6)?,
                filename: row.get(7)?,
                embedded_genre: row.get(8)?,
                bpm: row.get(9)?,
            }),
        )
        .optional()?
        .ok_or_else(|| AppError::Genre(format!("трек {track_id} не найден")))
}

fn infer(
    track: &TrackSignals,
    audio_predictions: Option<&[ApprovedGenrePrediction]>,
) -> Vec<GenreCandidate> {
    let mut scores: HashMap<(String, Option<String>), Aggregate> = HashMap::new();
    if let Some(tag) = &track.embedded_genre {
        for value in tag.split([';', ',', '|']) {
            if let Some((genre, subgenre)) = normalize(value) {
                add(
                    &mut scores,
                    genre,
                    subgenre,
                    0.94,
                    format!("Тег файла: {}", value.trim()),
                );
            }
        }
    }

    if let Some(predictions) = audio_predictions {
        for prediction in predictions {
            add(
                &mut scores,
                &prediction.genre,
                None,
                0.70 + 0.18 * prediction.score,
                format!(
                    "Локальная аудиомодель {} (критерий выпуска выполнен, оценка {:.2})",
                    prediction.model_version, prediction.score
                ),
            );
        }
    }

    let location = format!("{} {}", track.path, track.filename).to_lowercase();
    for (alias, genre, subgenre) in aliases() {
        if location.contains(alias) {
            add(
                &mut scores,
                genre,
                subgenre,
                0.66,
                format!("Название папки или файла содержит «{alias}»"),
            );
        }
    }

    add_bpm_signals(&mut scores, track.bpm);
    for (genre, score) in [("Techno", 0.14), ("House", 0.12), ("Drum & Bass", 0.10)] {
        add(
            &mut scores,
            genre,
            None,
            score,
            "Резервный кандидат таксономии".into(),
        );
    }

    let mut candidates = scores
        .into_iter()
        .map(|((genre, subgenre), aggregate)| GenreCandidate {
            rank: 0,
            genre,
            subgenre,
            score: (aggregate.score * 100.0).round() / 100.0,
            signals: aggregate.signals,
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        right
            .score
            .total_cmp(&left.score)
            .then_with(|| left.genre.cmp(&right.genre))
    });
    candidates.truncate(3);
    for (index, candidate) in candidates.iter_mut().enumerate() {
        candidate.rank = (index + 1) as u8;
    }
    candidates
}

fn add_bpm_signals(scores: &mut HashMap<(String, Option<String>), Aggregate>, bpm: Option<f64>) {
    let Some(bpm) = bpm.filter(|value| (40.0..=250.0).contains(value)) else {
        return;
    };
    let reason = |label: &str| format!("BPM {bpm:.1}: слабый ритмический сигнал для {label}");
    if bpm >= 165.0 {
        add(scores, "Drum & Bass", None, 0.38, reason("Drum & Bass"));
        add(scores, "Hardcore", None, 0.32, reason("Hardcore"));
        add(scores, "Hardstyle", None, 0.27, reason("Hardstyle"));
    } else if bpm >= 150.0 {
        add(
            scores,
            "Techno",
            Some("Hard Techno"),
            0.36,
            reason("Hard Techno"),
        );
        add(scores, "Hardstyle", None, 0.30, reason("Hardstyle"));
        add(scores, "Techno", None, 0.26, reason("Techno"));
    } else if bpm >= 135.0 {
        add(scores, "Techno", None, 0.34, reason("Techno"));
        add(scores, "Trance", None, 0.28, reason("Trance"));
        add(
            scores,
            "Techno",
            Some("Peak Time Techno"),
            0.25,
            reason("Peak Time Techno"),
        );
    } else if bpm >= 118.0 {
        add(scores, "House", None, 0.31, reason("House"));
        add(scores, "Techno", None, 0.27, reason("Techno"));
        add(scores, "Trance", None, 0.22, reason("Trance"));
    } else {
        add(scores, "Breaks", None, 0.24, reason("Breaks"));
        add(scores, "House", None, 0.20, reason("House"));
    }
}

fn add(
    scores: &mut HashMap<(String, Option<String>), Aggregate>,
    genre: &str,
    subgenre: Option<&str>,
    score: f64,
    signal: String,
) {
    let aggregate = scores
        .entry((genre.into(), subgenre.map(str::to_owned)))
        .or_default();
    if !aggregate.signals.contains(&signal) {
        aggregate.signals.push(signal);
    }
    if score > aggregate.score {
        aggregate.score = score;
    } else if score > 0.2 {
        aggregate.score = (aggregate.score + 0.04 * (1.0 - aggregate.score)).min(0.99);
    }
}

fn persist(
    state: &AppState,
    track_id: &str,
    candidates: &[GenreCandidate],
    inference_version: &str,
) -> Result<(), AppError> {
    let mut connection = state.connection()?;
    let transaction = connection.transaction()?;
    transaction.execute(
        "DELETE FROM track_genre_candidates WHERE track_id = ?1",
        [track_id],
    )?;
    for candidate in candidates {
        transaction.execute(
            "INSERT INTO track_genre_candidates(id, track_id, rank, genre, subgenre, score, signals_json, inference_version, taxonomy_version) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![Uuid::new_v4().to_string(), track_id, candidate.rank, candidate.genre, candidate.subgenre, candidate.score, serde_json::to_string(&candidate.signals).map_err(|error| AppError::Genre(error.to_string()))?, inference_version, TAXONOMY_VERSION],
        )?;
    }
    transaction.commit()?;
    Ok(())
}

fn normalize(value: &str) -> Option<(&'static str, Option<&'static str>)> {
    let value = value.trim().to_lowercase().replace('’', "'");
    aliases()
        .into_iter()
        .find(|(alias, _, _)| *alias == value)
        .map(|(_, genre, subgenre)| (genre, subgenre))
}

fn aliases() -> Vec<(&'static str, &'static str, Option<&'static str>)> {
    vec![
        ("breacks", "Breaks", None),
        ("break", "Breaks", None),
        ("breaks", "Breaks", None),
        ("dnb", "Drum & Bass", None),
        ("d&b", "Drum & Bass", None),
        ("drum'n'bass", "Drum & Bass", None),
        ("drum and bass", "Drum & Bass", None),
        ("drum & bass", "Drum & Bass", None),
        ("house", "House", None),
        ("techno", "Techno", None),
        ("trance", "Trance", None),
        ("hardcore", "Hardcore", None),
        ("hardstyle", "Hardstyle", None),
        ("electropunk", "Electropunk", None),
        ("electro punk", "Electropunk", None),
        ("melodic techno", "Techno", Some("Melodic Techno")),
        ("hard techno", "Techno", Some("Hard Techno")),
        ("percussion techno", "Techno", Some("Percussive Techno")),
        ("percussive techno", "Techno", Some("Percussive Techno")),
        ("pictime techno", "Techno", Some("Peak Time Techno")),
        ("peak time techno", "Techno", Some("Peak Time Techno")),
        ("acid techno", "Techno", Some("Acid Techno")),
        ("raw techno", "Techno", Some("Raw Techno")),
        ("schranz", "Techno", Some("Schranz")),
    ]
}

pub(crate) fn validate(genre: &str, subgenre: Option<&str>) -> Result<(), AppError> {
    if !BROAD_GENRES.contains(&genre) {
        return Err(AppError::Genre(
            "жанр отсутствует в таксономии версии 1".into(),
        ));
    }
    match subgenre {
        Some(value) if genre != "Techno" || !TECHNO_SUBGENRES.contains(&value) => Err(
            AppError::Genre("поджанр не соответствует выбранному жанру".into()),
        ),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio_metadata::AudioMetadata;

    #[test]
    fn embedded_alias_beats_weak_bpm_signal_and_confirmation_is_recorded() {
        let state = AppState::in_memory().expect("database");
        let root = state.register_root("D:\\Music").expect("root");
        let metadata = AudioMetadata {
            original_genre_tag: Some("acid techno".into()),
            bpm: Some(128.0),
            ..AudioMetadata::default()
        };
        state
            .upsert_file(
                &root,
                "D:\\Music\\Unknown.flac",
                "Unknown.flac",
                "flac",
                42,
                10,
                "genre-test",
                &metadata,
                "available",
            )
            .expect("track");
        let track_id = state.list_tracks(1, 0, "").expect("tracks")[0].id.clone();
        let suggestions = suggest(&state, std::slice::from_ref(&track_id)).expect("suggestions");
        assert_eq!(suggestions[0].candidates.len(), 3);
        assert_eq!(suggestions[0].candidates[0].genre, "Techno");
        assert_eq!(
            suggestions[0].candidates[0].subgenre.as_deref(),
            Some("Acid Techno")
        );
        assert!(suggestions[0].candidates[0].score > 0.9);

        confirm(&state, &track_id, "Techno", Some("Acid Techno"), true).expect("confirm");
        let connection = state.connection().expect("connection");
        let saved: (String, String, bool) = connection
            .query_row(
                "SELECT confirmed_genre, confirmed_subgenre, genre_locked FROM tracks WHERE id = ?1",
                [&track_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("saved genre");
        assert_eq!(saved, ("Techno".into(), "Acid Techno".into(), true));
        drop(connection);
        assert!(confirm(&state, &track_id, "House", None, false).is_err());
        confirm(&state, &track_id, "Techno", Some("Acid Techno"), false).expect("unlock");
        let connection = state.connection().expect("connection");
        let history: u64 = connection
            .query_row(
                "SELECT count(*) FROM genre_confirmation_history",
                [],
                |row| row.get(0),
            )
            .expect("history");
        assert_eq!(history, 2);
    }

    #[test]
    fn rejects_subgenre_outside_its_parent() {
        let state = AppState::in_memory().expect("database");
        assert!(confirm(&state, "missing", "House", Some("Acid Techno"), false).is_err());
    }
}

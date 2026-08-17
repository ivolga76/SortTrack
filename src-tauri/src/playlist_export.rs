use std::{
    fs::OpenOptions,
    io::Write,
    path::{Component, Path, PathBuf},
};

use rusqlite::OptionalExtension;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{
    error::AppError,
    models::{
        PlaylistExportPreview, PlaylistExportPreviewItem, PlaylistExportReport,
        PlaylistExportRequest,
    },
    repository::AppState,
};

#[derive(Debug, Clone)]
struct ExportTrack {
    position: u64,
    title: String,
    artist: String,
    album: String,
    genre: String,
    bpm: Option<f64>,
    musical_key: String,
    duration_seconds: u64,
    source_path: String,
}

struct PreparedExport {
    preview: PlaylistExportPreview,
    content: Vec<u8>,
}

pub fn preview(
    state: &AppState,
    request: PlaylistExportRequest,
) -> Result<PlaylistExportPreview, AppError> {
    Ok(prepare(state, request)?.preview)
}

pub fn commit(
    state: &AppState,
    request: PlaylistExportRequest,
    expected_state: &str,
) -> Result<PlaylistExportReport, AppError> {
    let prepared = prepare(state, request)?;
    if prepared.preview.expected_state != expected_state {
        return Err(AppError::InvalidPlaylist(
            "playlist or export settings changed after preview".into(),
        ));
    }
    let destination = Path::new(&prepared.preview.destination_path);
    if destination.exists() {
        return Err(AppError::InvalidPlaylist(
            "export destination already exists; overwriting is disabled".into(),
        ));
    }
    let parent = destination
        .parent()
        .ok_or_else(|| AppError::InvalidPlaylist("export destination has no parent".into()))?;
    let temporary = parent.join(format!(".sorttrack-export-{}.tmp", Uuid::new_v4()));
    let write_result = (|| -> Result<(), AppError> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        file.write_all(&prepared.content)?;
        file.sync_all()?;
        drop(file);
        if destination.exists() {
            return Err(AppError::InvalidPlaylist(
                "export destination appeared during writing".into(),
            ));
        }
        std::fs::rename(&temporary, destination)?;
        Ok(())
    })();
    if write_result.is_err() && temporary.exists() {
        let _ = std::fs::remove_file(&temporary);
    }
    write_result?;

    Ok(PlaylistExportReport {
        output_path: prepared.preview.destination_path,
        format: prepared.preview.format.clone(),
        total_tracks: prepared.preview.total_tracks,
        missing_tracks: prepared.preview.missing_tracks,
        bytes_written: prepared.content.len() as u64,
        instructions: if prepared.preview.format == "traktor_nml" {
            "В Traktor откройте Browser, щёлкните правой кнопкой по Playlists и выберите Import Playlist, затем укажите созданный NML. Не заменяйте collection.nml.".into()
        } else {
            "Импортируйте созданный M3U8 через команду импорта плейлиста вашего DJ-приложения."
                .into()
        },
    })
}

fn prepare(state: &AppState, request: PlaylistExportRequest) -> Result<PreparedExport, AppError> {
    validate_request(&request)?;
    let raw_destination = PathBuf::from(&request.destination_path);
    let filename = raw_destination
        .file_name()
        .ok_or_else(|| AppError::InvalidPlaylist("export filename is missing".into()))?;
    let parent = raw_destination
        .parent()
        .ok_or_else(|| AppError::InvalidPlaylist("export destination has no parent".into()))?
        .canonicalize()?;
    if !parent.is_dir() {
        return Err(AppError::InvalidPlaylist(
            "export destination directory is unavailable".into(),
        ));
    }
    let destination = parent.join(filename);
    let (playlist_name, tracks) = load_playlist(state, &request.playlist_id)?;
    if tracks.is_empty() {
        return Err(AppError::InvalidPlaylist("playlist has no tracks".into()));
    }
    let output_locations = tracks
        .iter()
        .map(|track| {
            if request.format == "m3u8" && request.path_mode == "relative" {
                let source = Path::new(&track.source_path);
                let normalized_source = source
                    .canonicalize()
                    .unwrap_or_else(|_| source.to_path_buf());
                relative_path(&parent, &normalized_source)
                    .unwrap_or_else(|| track.source_path.clone())
            } else {
                track.source_path.clone()
            }
        })
        .collect::<Vec<_>>();
    let content = if request.format == "m3u8" {
        render_m3u8(&tracks, &output_locations).into_bytes()
    } else {
        render_nml(
            &playlist_name,
            &tracks,
            request.target_profile.as_deref().unwrap_or("traktor_pro_4"),
        )
        .into_bytes()
    };
    let mut hasher = Sha256::new();
    hasher.update(&content);
    hasher.update(destination.to_string_lossy().as_bytes());
    hasher.update(request.format.as_bytes());
    hasher.update(request.path_mode.as_bytes());
    if let Some(profile) = &request.target_profile {
        hasher.update(profile.as_bytes());
    }
    for track in &tracks {
        hasher.update([Path::new(&track.source_path).is_file() as u8]);
    }
    let expected_state = hex(&hasher.finalize());
    let items = tracks
        .iter()
        .zip(output_locations)
        .map(|(track, output_location)| {
            let exists = Path::new(&track.source_path).is_file();
            PlaylistExportPreviewItem {
                position: track.position,
                title: track.title.clone(),
                artist: track.artist.clone(),
                source_path: track.source_path.clone(),
                output_location,
                status: if exists { "ready" } else { "missing" }.into(),
                reason: if exists {
                    "Будет добавлен в экспорт"
                } else {
                    "Исходный файл отсутствует; запись сохранится в плейлисте"
                }
                .into(),
            }
        })
        .collect::<Vec<_>>();
    let missing_tracks = items.iter().filter(|item| item.status == "missing").count() as u64;
    Ok(PreparedExport {
        preview: PlaylistExportPreview {
            playlist_id: request.playlist_id,
            playlist_name,
            destination_path: destination.to_string_lossy().into_owned(),
            format: request.format,
            path_mode: request.path_mode,
            target_profile: request.target_profile,
            expected_state,
            total_tracks: items.len() as u64,
            missing_tracks,
            destination_exists: destination.exists(),
            items,
        },
        content,
    })
}

fn validate_request(request: &PlaylistExportRequest) -> Result<(), AppError> {
    if !matches!(request.format.as_str(), "m3u8" | "traktor_nml") {
        return Err(AppError::InvalidPlaylist(
            "unsupported export format".into(),
        ));
    }
    if !matches!(request.path_mode.as_str(), "absolute" | "relative") {
        return Err(AppError::InvalidPlaylist(
            "unsupported export path mode".into(),
        ));
    }
    let extension = Path::new(&request.destination_path)
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    if (request.format == "m3u8" && !extension.eq_ignore_ascii_case("m3u8"))
        || (request.format == "traktor_nml" && !extension.eq_ignore_ascii_case("nml"))
    {
        return Err(AppError::InvalidPlaylist(
            "export filename extension does not match format".into(),
        ));
    }
    if request.format == "traktor_nml" {
        if request.path_mode != "absolute" {
            return Err(AppError::InvalidPlaylist(
                "Traktor NML requires absolute paths".into(),
            ));
        }
        if !matches!(
            request.target_profile.as_deref(),
            Some("traktor_pro_3" | "traktor_pro_4")
        ) {
            return Err(AppError::InvalidPlaylist(
                "unsupported Traktor target profile".into(),
            ));
        }
    }
    Ok(())
}

fn load_playlist(
    state: &AppState,
    playlist_id: &str,
) -> Result<(String, Vec<ExportTrack>), AppError> {
    let connection = state.connection()?;
    let name: String = connection
        .query_row(
            "SELECT name FROM playlists WHERE id = ?1",
            [playlist_id],
            |row| row.get(0),
        )
        .optional()?
        .ok_or_else(|| AppError::InvalidPlaylist("playlist not found".into()))?;
    let mut statement = connection.prepare(
        "SELECT pi.position, coalesce(m.title, t.preferred_title, f.filename), coalesce(m.artist, t.preferred_artist, ''), coalesce(m.album, ''), coalesce(m.original_genre_tag, t.confirmed_genre, ''), m.bpm, coalesce(m.musical_key, ''), coalesce(f.duration_ms, 0), f.canonical_path FROM playlist_items pi JOIN tracks t ON t.id = pi.track_id JOIN track_files f ON f.id = (SELECT preferred.id FROM track_files preferred WHERE preferred.track_id = t.id ORDER BY CASE preferred.availability_status WHEN 'available' THEN 0 ELSE 1 END, CASE lower(preferred.extension) WHEN 'flac' THEN 0 ELSE 1 END LIMIT 1) LEFT JOIN track_metadata m ON m.track_file_id = f.id WHERE pi.playlist_id = ?1 ORDER BY pi.position, pi.id",
    )?;
    let tracks = statement
        .query_map([playlist_id], |row| {
            let duration_ms: u64 = row.get(7)?;
            Ok(ExportTrack {
                position: row.get(0)?,
                title: row.get(1)?,
                artist: row.get(2)?,
                album: row.get(3)?,
                genre: row.get(4)?,
                bpm: row.get(5)?,
                musical_key: row.get(6)?,
                duration_seconds: duration_ms / 1000,
                source_path: row.get(8)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok((name, tracks))
}

fn render_m3u8(tracks: &[ExportTrack], locations: &[String]) -> String {
    let mut output = String::from("#EXTM3U\r\n");
    for (track, location) in tracks.iter().zip(locations) {
        output.push_str(&format!(
            "#EXTINF:{},{} - {}\r\n{}\r\n",
            track.duration_seconds,
            track.artist.replace(['\r', '\n'], " "),
            track.title.replace(['\r', '\n'], " "),
            location
        ));
    }
    output
}

fn render_nml(playlist_name: &str, tracks: &[ExportTrack], _profile: &str) -> String {
    let mut output = String::new();
    output.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"no\"?>\r\n<NML VERSION=\"19\">\r\n  <HEAD COMPANY=\"www.native-instruments.com\" PROGRAM=\"Traktor\"/>\r\n");
    output.push_str(&format!("  <COLLECTION ENTRIES=\"{}\">\r\n", tracks.len()));
    for track in tracks {
        let location = nml_location(Path::new(&track.source_path));
        output.push_str(&format!(
            "    <ENTRY TITLE=\"{}\" ARTIST=\"{}\">\r\n      <LOCATION DIR=\"{}\" FILE=\"{}\" VOLUME=\"{}\" VOLUMEID=\"\"/>\r\n      <ALBUM TITLE=\"{}\"/>\r\n      <INFO GENRE=\"{}\" KEY=\"{}\"/>\r\n{}    </ENTRY>\r\n",
            xml(&track.title), xml(&track.artist), xml(&location.1), xml(&location.2), xml(&location.0), xml(&track.album), xml(&track.genre), xml(&track.musical_key),
            track.bpm.map(|bpm| format!("      <TEMPO BPM=\"{bpm:.3}\"/>\r\n")).unwrap_or_default()
        ));
    }
    output.push_str("  </COLLECTION>\r\n  <SETS ENTRIES=\"0\"/>\r\n  <PLAYLISTS>\r\n    <NODE TYPE=\"FOLDER\" NAME=\"$ROOT\">\r\n      <SUBNODES COUNT=\"1\">\r\n");
    output.push_str(&format!("        <NODE TYPE=\"PLAYLIST\" NAME=\"{}\">\r\n          <PLAYLIST ENTRIES=\"{}\" TYPE=\"LIST\">\r\n", xml(playlist_name), tracks.len()));
    for track in tracks {
        output.push_str(&format!(
            "            <ENTRY><PRIMARYKEY TYPE=\"TRACK\" KEY=\"{}\"/></ENTRY>\r\n",
            xml(&nml_primary_key(Path::new(&track.source_path)))
        ));
    }
    output.push_str("          </PLAYLIST>\r\n        </NODE>\r\n      </SUBNODES>\r\n    </NODE>\r\n  </PLAYLISTS>\r\n</NML>\r\n");
    output
}

fn nml_location(path: &Path) -> (String, String, String) {
    let text = path.to_string_lossy().replace('\\', "/");
    let (volume, rest) = if text.len() >= 2 && text.as_bytes()[1] == b':' {
        (
            text[..2].to_owned(),
            text[2..].trim_start_matches('/').to_owned(),
        )
    } else {
        (String::new(), text.trim_start_matches('/').to_owned())
    };
    let file = Path::new(&rest)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_owned();
    let directory = rest.strip_suffix(&file).unwrap_or_default();
    let directory = if directory.is_empty() {
        "/".to_owned()
    } else {
        format!("/:{}", directory.replace('/', "/:"))
    };
    (volume, directory, file)
}

fn nml_primary_key(path: &Path) -> String {
    let (volume, directory, file) = nml_location(path);
    if directory.ends_with(':') {
        format!("{volume}{directory}{file}")
    } else {
        format!("{volume}{directory}:{file}")
    }
}

fn relative_path(base: &Path, target: &Path) -> Option<String> {
    let base_parts = base.components().collect::<Vec<_>>();
    let target_parts = target.components().collect::<Vec<_>>();
    let mut common = 0usize;
    while common < base_parts.len()
        && common < target_parts.len()
        && component_key(base_parts[common]) == component_key(target_parts[common])
    {
        common += 1;
    }
    if common == 0 || matches!(base_parts.first(), Some(Component::Prefix(_))) && common == 1 {
        return None;
    }
    let mut result = PathBuf::new();
    for _ in common..base_parts.len() {
        result.push("..");
    }
    for component in &target_parts[common..] {
        result.push(component.as_os_str());
    }
    Some(result.to_string_lossy().into_owned())
}

fn component_key(component: Component<'_>) -> String {
    component.as_os_str().to_string_lossy().to_ascii_lowercase()
}

fn xml(value: &str) -> String {
    value
        .chars()
        .filter(|character| {
            matches!(*character, '\u{9}' | '\u{a}' | '\u{d}') || *character >= '\u{20}'
        })
        .collect::<String>()
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio_metadata::AudioMetadata;

    fn golden_track() -> ExportTrack {
        ExportTrack {
            position: 0,
            title: "A < B".into(),
            artist: "Artist & One".into(),
            album: "Album \"Prime\"".into(),
            genre: "Techno".into(),
            bpm: Some(128.5),
            musical_key: "F minor".into(),
            duration_seconds: 245,
            source_path: "C:\\Music\\Artist & One - A < B.mp3".into(),
        }
    }

    fn export_test_state() -> (AppState, PathBuf, PathBuf, String) {
        let root = std::env::temp_dir().join(format!("sorttrack-export-{}", Uuid::new_v4()));
        let source_dir = root.join("source");
        let export_dir = root.join("exports");
        std::fs::create_dir_all(&source_dir).expect("source directory");
        std::fs::create_dir_all(&export_dir).expect("export directory");
        let state = AppState::in_memory().expect("database");
        let library_root = state
            .register_root(&source_dir.to_string_lossy())
            .expect("library root");
        let playlist_id = state.create_playlist("Test Set").expect("playlist");
        for (position, (filename, title, artist)) in [
            ("First.mp3", "First", "Artist A"),
            ("Second.flac", "Second", "Artist B"),
        ]
        .into_iter()
        .enumerate()
        {
            let path = source_dir.join(filename);
            std::fs::write(&path, format!("audio-{position}")).expect("audio fixture");
            let file_metadata = path.metadata().expect("metadata");
            let modified_at_ms = file_metadata
                .modified()
                .expect("modified")
                .duration_since(std::time::UNIX_EPOCH)
                .expect("epoch")
                .as_millis() as i64;
            state
                .upsert_file(
                    &library_root,
                    &path.to_string_lossy(),
                    filename,
                    Path::new(filename).extension().unwrap().to_str().unwrap(),
                    file_metadata.len(),
                    modified_at_ms,
                    &format!("export-file-{position}"),
                    &AudioMetadata {
                        title: Some(title.into()),
                        artist: Some(artist.into()),
                        duration_ms: Some((position as u64 + 1) * 60_000),
                        ..AudioMetadata::default()
                    },
                    "available",
                )
                .expect("track");
            let track_id = state.list_tracks(10, 0, title).expect("track query")[0]
                .id
                .clone();
            state
                .place_track_in_playlist(&playlist_id, position as u32, &track_id, None, None)
                .expect("playlist item");
        }
        (state, root, export_dir, playlist_id)
    }

    #[test]
    fn renders_deterministic_utf8_m3u8() {
        let track = golden_track();
        assert_eq!(
            render_m3u8(&[track], &["C:\\Music\\Track.mp3".into()]),
            "#EXTM3U\r\n#EXTINF:245,Artist & One - A < B\r\nC:\\Music\\Track.mp3\r\n"
        );
    }

    #[test]
    fn traktor_profiles_match_their_golden_files() {
        let track = golden_track();
        for (profile, golden) in [
            (
                "traktor_pro_3",
                include_str!("../tests/golden/traktor_pro_3.nml"),
            ),
            (
                "traktor_pro_4",
                include_str!("../tests/golden/traktor_pro_4.nml"),
            ),
        ] {
            let expected = golden.replace("\r\n", "\n").replace('\n', "\r\n");
            assert_eq!(
                render_nml("Friday & Night", std::slice::from_ref(&track), profile),
                expected
            );
        }
    }

    #[test]
    fn export_commit_is_atomic_and_never_overwrites() {
        let (state, root, export_dir, playlist_id) = export_test_state();
        let destination = export_dir.join("Test Set.m3u8");
        let request = PlaylistExportRequest {
            playlist_id,
            destination_path: destination.to_string_lossy().into_owned(),
            format: "m3u8".into(),
            path_mode: "relative".into(),
            target_profile: None,
        };
        let preview = preview(&state, request.clone()).expect("preview");
        assert_eq!(preview.total_tracks, 2);
        assert_eq!(preview.missing_tracks, 0);
        assert!(preview.items[0].output_location.starts_with(".."));
        let report = commit(&state, request.clone(), &preview.expected_state).expect("commit");
        assert!(destination.is_file());
        assert_eq!(report.total_tracks, 2);
        let original = std::fs::read(&destination).expect("export content");
        assert!(commit(&state, request, &preview.expected_state).is_err());
        assert_eq!(
            std::fs::read(&destination).expect("unchanged export"),
            original
        );
        assert!(
            std::fs::read_dir(&export_dir)
                .expect("export entries")
                .all(|entry| !entry
                    .expect("entry")
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".sorttrack-export-"))
        );
        std::fs::remove_dir_all(root).expect("cleanup");
    }
}

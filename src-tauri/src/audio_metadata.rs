use std::path::Path;

use lofty::{
    file::{AudioFile, TaggedFileExt},
    probe::Probe,
    tag::{Accessor, ItemKey},
};

use crate::models::MetadataValues;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct AudioMetadata {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub album_artist: Option<String>,
    pub track_number: Option<String>,
    pub disc_number: Option<String>,
    pub original_genre_tag: Option<String>,
    pub year: Option<String>,
    pub composer: Option<String>,
    pub comment: Option<String>,
    pub bpm: Option<f64>,
    pub musical_key: Option<String>,
    pub camelot: Option<String>,
    pub duration_ms: Option<u64>,
    pub bitrate_kbps: Option<u32>,
    pub sample_rate_hz: Option<u32>,
    pub bit_depth: Option<u8>,
    pub channels: Option<u8>,
}

impl AudioMetadata {
    pub fn read(path: &Path) -> Result<Self, String> {
        let tagged_file = Probe::open(path)
            .map_err(|error| error.to_string())?
            .read()
            .map_err(|error| error.to_string())?;
        let properties = tagged_file.properties();
        let tag = tagged_file
            .primary_tag()
            .or_else(|| tagged_file.first_tag());

        let mut result = Self {
            duration_ms: Some(properties.duration().as_millis() as u64),
            bitrate_kbps: properties.overall_bitrate(),
            sample_rate_hz: properties.sample_rate(),
            bit_depth: properties.bit_depth(),
            channels: properties.channels(),
            ..Self::default()
        };

        if let Some(tag) = tag {
            result.apply_tag(tag);
        }

        Ok(result)
    }

    fn apply_tag(&mut self, tag: &lofty::tag::Tag) {
        self.title = owned(tag.title());
        self.artist = owned(tag.artist());
        self.album = owned(tag.album());
        self.album_artist = item(tag, ItemKey::AlbumArtist);
        self.track_number = tag.track().map(|value| value.to_string());
        self.disc_number = tag.disk().map(|value| value.to_string());
        self.original_genre_tag = owned(tag.genre());
        self.year = item(tag, ItemKey::RecordingDate).or_else(|| item(tag, ItemKey::Year));
        self.composer = item(tag, ItemKey::Composer);
        self.comment = owned(tag.comment());
        self.bpm = item(tag, ItemKey::IntegerBpm)
            .or_else(|| item(tag, ItemKey::Bpm))
            .and_then(|value| value.trim().parse().ok());
        self.musical_key = item(tag, ItemKey::InitialKey);
        self.camelot = self.musical_key.as_deref().and_then(camelot_for_key);
    }

    pub fn fingerprint(&self) -> String {
        format!(
            "{:?}|{:?}|{:?}|{:?}|{:?}|{:?}",
            self.title,
            self.artist,
            self.album,
            self.original_genre_tag,
            self.bpm,
            self.musical_key
        )
    }
}

impl From<&AudioMetadata> for MetadataValues {
    fn from(value: &AudioMetadata) -> Self {
        Self {
            title: value.title.clone(),
            artist: value.artist.clone(),
            album: value.album.clone(),
            album_artist: value.album_artist.clone(),
            track_number: value.track_number.clone(),
            disc_number: value.disc_number.clone(),
            genre: value.original_genre_tag.clone(),
            year: value.year.clone(),
            composer: value.composer.clone(),
            comment: value.comment.clone(),
            bpm: value.bpm,
            musical_key: value.musical_key.clone(),
            camelot: value.camelot.clone(),
        }
    }
}

impl From<&MetadataValues> for AudioMetadata {
    fn from(value: &MetadataValues) -> Self {
        Self {
            title: value.title.clone(),
            artist: value.artist.clone(),
            album: value.album.clone(),
            album_artist: value.album_artist.clone(),
            track_number: value.track_number.clone(),
            disc_number: value.disc_number.clone(),
            original_genre_tag: value.genre.clone(),
            year: value.year.clone(),
            composer: value.composer.clone(),
            comment: value.comment.clone(),
            bpm: value.bpm,
            musical_key: value.musical_key.clone(),
            camelot: value.camelot.clone(),
            ..Self::default()
        }
    }
}

fn owned(value: Option<std::borrow::Cow<'_, str>>) -> Option<String> {
    value.map(|value| value.into_owned())
}

fn item(tag: &lofty::tag::Tag, key: ItemKey) -> Option<String> {
    tag.get_string(key).map(str::to_owned)
}

pub fn camelot_for_key(value: &str) -> Option<String> {
    let normalized = value
        .trim()
        .to_lowercase()
        .replace('♯', "#")
        .replace('♭', "b")
        .replace("major", "maj")
        .replace("minor", "min")
        .replace(' ', "");
    let key = match normalized.as_str() {
        "abmin" | "g#min" => "1A",
        "bmaj" | "cbmaj" => "1B",
        "ebmin" | "d#min" => "2A",
        "f#maj" | "gbmaj" => "2B",
        "bbmin" | "a#min" => "3A",
        "dbmaj" | "c#maj" => "3B",
        "fmin" => "4A",
        "abmaj" | "g#maj" => "4B",
        "cmin" => "5A",
        "ebmaj" | "d#maj" => "5B",
        "gmin" => "6A",
        "bbmaj" | "a#maj" => "6B",
        "dmin" => "7A",
        "fmaj" => "7B",
        "amin" => "8A",
        "cmaj" => "8B",
        "emin" => "9A",
        "gmaj" => "9B",
        "bmin" => "10A",
        "dmaj" => "10B",
        "f#min" | "gbmin" => "11A",
        "amaj" => "11B",
        "c#min" | "dbmin" => "12A",
        "emaj" => "12B",
        _ => return None,
    };
    Some(key.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use lofty::tag::TagType;

    #[test]
    fn converts_common_key_spellings_to_camelot() {
        assert_eq!(camelot_for_key("A minor").as_deref(), Some("8A"));
        assert_eq!(camelot_for_key("C Major").as_deref(), Some("8B"));
        assert_eq!(camelot_for_key("G♯ min").as_deref(), Some("1A"));
        assert_eq!(camelot_for_key("unknown"), None);
    }

    #[test]
    fn extracts_dj_metadata_from_a_tag() {
        let mut tag = lofty::tag::Tag::new(TagType::Id3v2);
        tag.set_title("Night Drive".to_owned());
        tag.set_artist("SortTrack Test".to_owned());
        tag.insert_text(ItemKey::IntegerBpm, "142.5".to_owned());
        tag.insert_text(ItemKey::InitialKey, "A minor".to_owned());

        let mut metadata = AudioMetadata::default();
        metadata.apply_tag(&tag);

        assert_eq!(metadata.title.as_deref(), Some("Night Drive"));
        assert_eq!(metadata.artist.as_deref(), Some("SortTrack Test"));
        assert_eq!(metadata.bpm, Some(142.5));
        assert_eq!(metadata.camelot.as_deref(), Some("8A"));
    }
}

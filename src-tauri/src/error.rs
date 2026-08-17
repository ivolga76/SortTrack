use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("filesystem error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialization error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid library root: {0}")]
    InvalidLibraryRoot(String),
    #[error("application state is unavailable")]
    StateUnavailable,
    #[error("background task failed: {0}")]
    Task(String),
    #[error("track is unavailable for playback: {0}")]
    TrackUnavailable(String),
    #[error("invalid playlist operation: {0}")]
    InvalidPlaylist(String),
    #[error("metadata operation rejected: {0}")]
    Metadata(String),
    #[error("genre operation rejected: {0}")]
    Genre(String),
}

impl serde::Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("AppError", 4)?;
        state.serialize_field("schema", "sorttrack-error-1")?;
        state.serialize_field("code", self.code())?;
        state.serialize_field("message", &self.to_string())?;
        state.serialize_field("retryable", &self.retryable())?;
        state.end()
    }
}

impl AppError {
    fn code(&self) -> &'static str {
        match self {
            Self::Database(_) => "DATABASE",
            Self::Io(_) => "FILESYSTEM",
            Self::Json(_) => "SERIALIZATION",
            Self::StateUnavailable | Self::TrackUnavailable(_) => "UNAVAILABLE",
            Self::Task(message) if is_invalid_input(message) => "INVALID_INPUT",
            Self::Task(_) => "TASK_FAILED",
            Self::InvalidLibraryRoot(message)
            | Self::InvalidPlaylist(message)
            | Self::Metadata(message)
            | Self::Genre(message)
                if is_stale_state(message) =>
            {
                "STALE_STATE"
            }
            Self::InvalidLibraryRoot(message)
            | Self::InvalidPlaylist(message)
            | Self::Metadata(message)
            | Self::Genre(message)
                if is_not_found(message) =>
            {
                "NOT_FOUND"
            }
            Self::InvalidLibraryRoot(_)
            | Self::InvalidPlaylist(_)
            | Self::Metadata(_)
            | Self::Genre(_) => "INVALID_INPUT",
        }
    }

    fn retryable(&self) -> bool {
        matches!(
            self,
            Self::StateUnavailable | Self::TrackUnavailable(_) | Self::Task(_)
        )
    }
}

fn is_stale_state(message: &str) -> bool {
    let message = message.to_lowercase();
    message.contains("changed")
        || message.contains("stale")
        || message.contains("expected state")
        || message.contains("изменил")
        || message.contains("изменился")
        || message.contains("изменилась")
}

fn is_not_found(message: &str) -> bool {
    let message = message.to_lowercase();
    message.contains("not found")
        || message.contains("не найден")
        || message.contains("отсутствует")
}

fn is_invalid_input(message: &str) -> bool {
    let message = message.to_lowercase();
    message.contains("invalid")
        || message.contains("must ")
        || message.contains("cannot")
        || message.contains("некоррект")
}

#[cfg(test)]
mod tests {
    use super::AppError;

    #[test]
    fn serializes_a_versioned_stable_error_contract() {
        let value = serde_json::to_value(AppError::InvalidPlaylist(
            "playlist changed after preview".into(),
        ))
        .expect("serialize app error");
        assert_eq!(value["schema"], "sorttrack-error-1");
        assert_eq!(value["code"], "STALE_STATE");
        assert_eq!(
            value["message"],
            "invalid playlist operation: playlist changed after preview"
        );
        assert_eq!(value["retryable"], false);
    }

    #[test]
    fn keeps_unavailable_failures_retryable() {
        let value = serde_json::to_value(AppError::TrackUnavailable("track".into()))
            .expect("serialize app error");
        assert_eq!(value["code"], "UNAVAILABLE");
        assert_eq!(value["retryable"], true);
    }
}

use std::{
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

use serde::Serialize;
use uuid::Uuid;

use crate::error::AppError;

#[derive(Clone, Default)]
pub struct ScanRuntimeState {
    inner: Arc<Mutex<Option<ScanRuntimeJob>>>,
}

#[derive(Clone)]
struct ScanRuntimeJob {
    id: String,
    root_path: String,
    state: String,
    processed_items: u64,
    total_items: u64,
    started_at: Instant,
    paused_at: Option<Instant>,
    paused_duration: Duration,
    finished_at: Option<Instant>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanProgress {
    pub id: String,
    pub root_path: String,
    pub state: String,
    pub processed_items: u64,
    pub total_items: u64,
    pub items_per_second: Option<f64>,
    pub eta_seconds: Option<u64>,
    pub elapsed_seconds: u64,
}

impl From<&ScanRuntimeJob> for ScanProgress {
    fn from(job: &ScanRuntimeJob) -> Self {
        let now = job.finished_at.unwrap_or_else(Instant::now);
        let current_pause = job
            .paused_at
            .map(|started| now.saturating_duration_since(started))
            .unwrap_or_default();
        let elapsed = now
            .saturating_duration_since(job.started_at)
            .saturating_sub(job.paused_duration + current_pause);
        let elapsed_float = elapsed.as_secs_f64();
        let items_per_second = (job.processed_items > 0 && elapsed_float > 0.01)
            .then(|| job.processed_items as f64 / elapsed_float);
        let eta_seconds = items_per_second.filter(|rate| *rate > 0.0).map(|rate| {
            ((job.total_items.saturating_sub(job.processed_items)) as f64 / rate).ceil() as u64
        });
        Self {
            id: job.id.clone(),
            root_path: job.root_path.clone(),
            state: job.state.clone(),
            processed_items: job.processed_items,
            total_items: job.total_items,
            items_per_second,
            eta_seconds,
            elapsed_seconds: elapsed.as_secs(),
        }
    }
}

impl ScanRuntimeState {
    pub fn start(&self, root_path: String, total_items: u64) -> Result<String, AppError> {
        let mut current = self.inner.lock().map_err(|_| AppError::StateUnavailable)?;
        if current
            .as_ref()
            .is_some_and(|job| matches!(job.state.as_str(), "running" | "paused" | "cancelling"))
        {
            return Err(AppError::Task(
                "Another library scan is already active".into(),
            ));
        }
        let id = Uuid::new_v4().to_string();
        *current = Some(ScanRuntimeJob {
            id: id.clone(),
            root_path,
            state: "running".into(),
            processed_items: 0,
            total_items,
            started_at: Instant::now(),
            paused_at: None,
            paused_duration: Duration::ZERO,
            finished_at: None,
        });
        Ok(id)
    }

    pub fn checkpoint(&self, id: &str, processed_items: u64) -> Result<bool, AppError> {
        loop {
            let state = {
                let mut current = self.inner.lock().map_err(|_| AppError::StateUnavailable)?;
                let job = current
                    .as_mut()
                    .filter(|job| job.id == id)
                    .ok_or_else(|| AppError::Task("Active scan job was not found".into()))?;
                job.total_items = job.total_items.max(processed_items);
                job.processed_items = processed_items;
                job.state.clone()
            };
            match state.as_str() {
                "running" => return Ok(true),
                "paused" => thread::sleep(Duration::from_millis(75)),
                "cancelling" => return Ok(false),
                _ => return Ok(false),
            }
        }
    }

    pub fn finish(&self, id: &str, processed_items: u64, cancelled: bool) -> Result<(), AppError> {
        let mut current = self.inner.lock().map_err(|_| AppError::StateUnavailable)?;
        let job = current
            .as_mut()
            .filter(|job| job.id == id)
            .ok_or_else(|| AppError::Task("Active scan job was not found".into()))?;
        job.total_items = job.total_items.max(processed_items);
        job.processed_items = processed_items;
        if !cancelled {
            job.total_items = processed_items;
        }
        close_pause(job);
        job.finished_at = Some(Instant::now());
        job.state = if cancelled { "cancelled" } else { "completed" }.into();
        Ok(())
    }

    pub fn fail(&self, id: &str) {
        if let Ok(mut current) = self.inner.lock() {
            if let Some(job) = current.as_mut().filter(|job| job.id == id) {
                close_pause(job);
                job.finished_at = Some(Instant::now());
                job.state = "failed".into();
            }
        }
    }

    pub fn progress(&self) -> Result<Option<ScanProgress>, AppError> {
        let current = self.inner.lock().map_err(|_| AppError::StateUnavailable)?;
        Ok(current.as_ref().map(ScanProgress::from))
    }

    pub fn pause(&self, id: &str) -> Result<(), AppError> {
        let mut current = self.inner.lock().map_err(|_| AppError::StateUnavailable)?;
        let job = current
            .as_mut()
            .filter(|job| job.id == id)
            .ok_or_else(|| AppError::Task("Active scan job was not found".into()))?;
        if job.state == "running" {
            job.state = "paused".into();
            job.paused_at = Some(Instant::now());
        }
        Ok(())
    }
    pub fn resume(&self, id: &str) -> Result<(), AppError> {
        let mut current = self.inner.lock().map_err(|_| AppError::StateUnavailable)?;
        let job = current
            .as_mut()
            .filter(|job| job.id == id)
            .ok_or_else(|| AppError::Task("Active scan job was not found".into()))?;
        if job.state == "paused" {
            close_pause(job);
            job.state = "running".into();
        }
        Ok(())
    }

    pub fn cancel(&self, id: &str) -> Result<(), AppError> {
        let mut current = self.inner.lock().map_err(|_| AppError::StateUnavailable)?;
        let job = current
            .as_mut()
            .filter(|job| job.id == id)
            .ok_or_else(|| AppError::Task("Active scan job was not found".into()))?;
        if matches!(job.state.as_str(), "running" | "paused") {
            job.state = "cancelling".into();
        }
        Ok(())
    }
}

fn close_pause(job: &mut ScanRuntimeJob) {
    if let Some(started) = job.paused_at.take() {
        job.paused_duration += Instant::now().saturating_duration_since(started);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_runtime_pauses_resumes_and_cancels_at_checkpoints() {
        let runtime = ScanRuntimeState::default();
        let id = runtime.start("D:\\Music".into(), 10).unwrap();
        runtime.pause(&id).unwrap();
        assert_eq!(runtime.progress().unwrap().unwrap().state, "paused");
        runtime.resume(&id).unwrap();
        thread::sleep(Duration::from_millis(12));
        assert!(runtime.checkpoint(&id, 3).unwrap());
        let running = runtime.progress().unwrap().unwrap();
        assert!(running.items_per_second.is_some());
        assert!(running.eta_seconds.is_some());
        runtime.cancel(&id).unwrap();
        assert!(!runtime.checkpoint(&id, 4).unwrap());
        runtime.finish(&id, 4, true).unwrap();
        let progress = runtime.progress().unwrap().unwrap();
        assert_eq!(progress.state, "cancelled");
        assert_eq!(progress.processed_items, 4);
    }
}

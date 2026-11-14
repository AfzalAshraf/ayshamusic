//! Bounds for the existing compatibility metadata path, not native extraction.
//! This does not bound the helper's RSS or certify a whole-server memory budget.
use std::{process::Stdio, time::Duration};
use tokio::{io::AsyncReadExt, process::Command, sync::Semaphore};

const MAX_HELPER_BYTES: usize = 256 * 1024;
const MAX_HTTP_BYTES: usize = 512 * 1024;
static METADATA_SLOT: Semaphore = Semaphore::const_new(1);

#[derive(Default)]
pub struct PrefetchAttempts { epoch: Option<u64> }
impl PrefetchAttempts {
    pub fn reserve(&mut self, epoch: u64, playing: bool, queue_empty: bool) -> bool {
        if !playing || !queue_empty || self.epoch == Some(epoch) { return false; }
        self.epoch = Some(epoch);
        true
    }
}

pub async fn helper_output(command: Command) -> Result<Vec<u8>, &'static str> {
    run(command, &METADATA_SLOT, Duration::from_secs(30)).await
}

async fn run(mut command: Command, slots: &Semaphore, deadline: Duration) -> Result<Vec<u8>, &'static str> {
    // Do not accumulate waiting subprocess jobs or tie up blocking-pool threads.
    let _permit = slots.try_acquire().map_err(|_| "metadata_busy")?;
    let mut child = command.stdin(Stdio::null()).stdout(Stdio::piped())
        .stderr(Stdio::null()).kill_on_drop(true).spawn().map_err(|_| "metadata_spawn_failed")?;
    let Some(stdout) = child.stdout.take() else { return Err("metadata_stdout_missing") };
    let result = tokio::time::timeout(deadline, async {
        let mut bytes = Vec::with_capacity(8192);
        stdout.take((MAX_HELPER_BYTES + 1) as u64).read_to_end(&mut bytes)
            .await.map_err(|_| "metadata_read_failed")?;
        if bytes.len() > MAX_HELPER_BYTES { return Err("metadata_output_limit"); }
        let status = child.wait().await.map_err(|_| "metadata_wait_failed")?;
        if !status.success() { return Err("metadata_exit_failed"); }
        Ok(bytes)
    }).await.unwrap_or(Err("metadata_timeout"));
    if result.is_err() {
        // Kill/reap only this direct child; never global process-pattern killing.
        // Cancellation still uses kill_on_drop; helper grandchildren are not
        // independently managed or excluded from total memory accounting.
        let _ = tokio::time::timeout(Duration::from_secs(1), child.kill()).await;
    }
    result
}

pub async fn bounded_json(mut response: reqwest::Response) -> Option<serde_json::Value> {
    if !response.status().is_success() || response.content_length().is_some_and(|len| len > MAX_HTTP_BYTES as u64) {
        return None;
    }
    let mut bytes = Vec::with_capacity(8192);
    while let Some(chunk) = response.chunk().await.ok()? {
        if chunk.len() > MAX_HTTP_BYTES.saturating_sub(bytes.len()) { return None; }
        bytes.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&bytes).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn shell(script: &str) -> Command {
        let mut command = Command::new("sh"); command.args(["-c",script]); command
    }
    #[test]
    fn prefetch_is_once_per_play_epoch_even_after_failure() {
        let mut attempts = PrefetchAttempts::default();
        assert!(!attempts.reserve(1, false, true));
        assert!(!attempts.reserve(1, true, false));
        assert!(attempts.reserve(1, true, true));
        assert!(!attempts.reserve(1, true, true));
        assert!(attempts.reserve(2, true, true));
        assert!(!attempts.reserve(2, true, true));
    }
    #[tokio::test]
    async fn bounded_helper_accepts_small_output_and_releases_slot_after_exit() {
        let slots = Semaphore::new(1);
        assert_eq!(run(shell("printf fixture"), &slots, Duration::from_secs(3)).await.unwrap(), b"fixture");
        assert_eq!(slots.available_permits(), 1);
        assert_eq!(run(shell("exit 3"), &slots, Duration::from_secs(3)).await, Err("metadata_exit_failed"));
        assert_eq!(slots.available_permits(), 1);
    }
    #[tokio::test]
    async fn busy_slot_never_starts_the_requested_command() {
        let slots = Semaphore::new(1); let _held = slots.acquire().await.unwrap();
        assert_eq!(run(Command::new("/no-such-aysha-test-program"), &slots, Duration::from_secs(3)).await,
                   Err("metadata_busy"));
    }
    #[tokio::test]
    async fn oversized_output_and_timeout_are_failures_and_release_slot() {
        let slots = Semaphore::new(1);
        assert_eq!(run(shell("head -c 262145 /dev/zero"), &slots, Duration::from_secs(3)).await,
                   Err("metadata_output_limit"));
        assert_eq!(run(Command::new("/no-such-aysha-test-program"), &slots, Duration::from_secs(3)).await,
                   Err("metadata_spawn_failed"));
        let mut sleeper = Command::new("sleep"); sleeper.arg("30");
        assert_eq!(run(sleeper, &slots, Duration::from_millis(50)).await, Err("metadata_timeout"));
        assert_eq!(slots.available_permits(), 1);
    }
    #[tokio::test]
    async fn cancelled_task_releases_slot() {
        let slots = std::sync::Arc::new(Semaphore::new(1));
        let worker_slots = slots.clone();
        let task = tokio::spawn(async move {
            let mut sleeper = Command::new("sleep"); sleeper.arg("30");
            run(sleeper, &worker_slots, Duration::from_secs(30)).await
        });
        tokio::time::timeout(Duration::from_secs(3), async {
            while slots.available_permits() != 0 { tokio::task::yield_now().await; }
        }).await.unwrap();
        task.abort(); let _ = task.await;
        assert_eq!(slots.available_permits(), 1);
    }
    #[tokio::test]
    async fn http_json_is_bounded_and_error_status_is_not_metadata() {
        let good = reqwest::Response::from(axum::http::Response::new(b"{\"fixture\":true}".to_vec()));
        assert_eq!(bounded_json(good).await.unwrap()["fixture"], true);
        let oversized = reqwest::Response::from(axum::http::Response::new(vec![b' '; MAX_HTTP_BYTES + 1]));
        assert!(bounded_json(oversized).await.is_none());
        let error = reqwest::Response::from(axum::http::Response::builder().status(403).body(b"{}".to_vec()).unwrap());
        assert!(bounded_json(error).await.is_none());
    }
}

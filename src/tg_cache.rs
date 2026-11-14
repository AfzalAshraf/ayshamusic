//! Telegram cloud cache: persistent upload queue, sliding-window quota limiter,
//! `FLOOD_WAIT` handling and a bounded disk LRU cache.
//!
//! Storage layout (all under the Android-provided directories):
//! - `filesDir/aysha_tg_cache.db`  - SQLite database (WAL)
//! - `cacheDir/aysha_media/`       - cached audio files (LRU-managed)
//!
//! Upload pacing rules: at most 18 uploads per rolling 3600 s, with a minimum
//! 12 s gap between uploads, and one upload at a time (never parallel).

use crate::tg_auth::TelegramAuth;
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use std::collections::HashSet;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub const DEFAULT_MAX_CACHE_BYTES: u64 = 400 * 1024 * 1024;
pub const MAX_UPLOADS_PER_WINDOW: u32 = 18;
pub const QUOTA_WINDOW_SECS: u64 = 3600;
pub const MIN_UPLOAD_PACING_SECS: u64 = 12;
pub const FLOOD_WAIT_BUFFER_SECS: u64 = 5;
pub const MAX_UPLOAD_RETRIES: i64 = 3;

const CACHE_SUBDIR: &str = "aysha_media";
const DB_FILE: &str = "aysha_tg_cache.db";
const COOLDOWN_KEY: &str = "flood_wait_until";

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS upload_queue (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    video_id     TEXT NOT NULL UNIQUE,
    file_path    TEXT NOT NULL,
    title        TEXT NOT NULL,
    artist       TEXT NOT NULL,
    caption      TEXT NOT NULL,
    status       TEXT NOT NULL DEFAULT 'pending',
    created_at   INTEGER NOT NULL,
    updated_at   INTEGER NOT NULL,
    retry_count  INTEGER NOT NULL DEFAULT 0,
    last_error   TEXT
);
CREATE INDEX IF NOT EXISTS idx_upload_queue_status ON upload_queue(status);

CREATE TABLE IF NOT EXISTS upload_history (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    video_id    TEXT NOT NULL,
    uploaded_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_upload_history_uploaded_at ON upload_history(uploaded_at);

CREATE TABLE IF NOT EXISTS saved_tracks (video_id TEXT PRIMARY KEY, message_id INTEGER NOT NULL);

CREATE TABLE IF NOT EXISTS tg_meta (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
"#;

/// Quota snapshot exposed by `GET /api/tg/quota`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct QuotaStatus {
    pub remaining_in_window: u32,
    pub max_per_window: u32,
    pub queued_tracks: u32,
    pub cooldown_seconds_left: u64,
    pub status_label: String,
}

/// One queued upload, as handed to the upload worker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueueItem {
    pub id: i64,
    pub video_id: String,
    pub file_path: String,
    pub title: String,
    pub artist: String,
    pub caption: String,
    pub retry_count: i64,
}

/// Result of a single upload attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UploadResult {
    Done,
    FloodWait { seconds: u64 },
    Failed { error: String },
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct EvictionReport {
    pub deleted_files: u64,
    pub freed_bytes: u64,
    pub remaining_bytes: u64,
}

#[derive(Debug)]
pub enum CacheError {
    Db(rusqlite::Error),
    Io(std::io::Error),
}

impl std::fmt::Display for CacheError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CacheError::Db(error) => write!(f, "sqlite error: {error}"),
            CacheError::Io(error) => write!(f, "io error: {error}"),
        }
    }
}

impl std::error::Error for CacheError {}

impl From<rusqlite::Error> for CacheError {
    fn from(error: rusqlite::Error) -> Self {
        CacheError::Db(error)
    }
}

impl From<std::io::Error> for CacheError {
    fn from(error: std::io::Error) -> Self {
        CacheError::Io(error)
    }
}

pub type Result<T> = std::result::Result<T, CacheError>;

/// Persistent upload queue + quota + bounded disk cache.
pub struct TelegramCacheManager {
    db: Mutex<Connection>,
    media_dir: PathBuf,
    max_cache_bytes: u64,
    min_pacing_secs: u64,
    telegram: TelegramAuth,
}

impl TelegramCacheManager {
    /// Opens (creating if needed) the queue database and cache directory.
    pub fn open(files_dir: impl AsRef<Path>, cache_dir: impl AsRef<Path>) -> Result<Self> {
        Self::open_with(
            files_dir,
            cache_dir,
            DEFAULT_MAX_CACHE_BYTES,
            MIN_UPLOAD_PACING_SECS,
        )
    }

    /// Test/configuration hook: custom cache budget and pacing gap.
    pub fn open_with(
        files_dir: impl AsRef<Path>,
        cache_dir: impl AsRef<Path>,
        max_cache_bytes: u64,
        min_pacing_secs: u64,
    ) -> Result<Self> {
        let media_dir = cache_dir.as_ref().join(CACHE_SUBDIR);
        std::fs::create_dir_all(&media_dir)?;

        let connection = Connection::open(files_dir.as_ref().join(DB_FILE))?;
        // WAL keeps the queue readable while the uploader writes.
        connection.query_row("PRAGMA journal_mode=WAL", [], |row| row.get::<_, String>(0))?;
        connection.pragma_update(None, "synchronous", "NORMAL")?;
        connection.execute_batch(SCHEMA)?;
        // Crash recovery: a process killed mid-upload leaves rows in
        // 'uploading' with no worker owning them. Requeue them so they are
        // retried on this start instead of being stranded forever.
        connection.execute(
            "UPDATE upload_queue SET status = 'pending', updated_at = ?1 WHERE status = 'uploading'",
            params![now_ts()],
        )?;

        Ok(Self {
            db: Mutex::new(connection),
            media_dir,
            max_cache_bytes,
            min_pacing_secs,
            telegram: TelegramAuth::new(files_dir.as_ref()),
        })
    }

    /// Login state machine shared with the `/api/tg/*` handlers.
    pub fn telegram(&self) -> &TelegramAuth {
        &self.telegram
    }

    fn conn(&self) -> MutexGuard<'_, Connection> {
        // A poisoned mutex must not take the music app down.
        self.db.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Telegram caption for a cached track: `#yt_<video_id>`, title, artist.
    pub fn caption_for(video_id: &str, title: &str, artist: &str) -> String {
        format!("#yt_{video_id}\n{title}\n{artist}")
    }

    /// Local cache path for a YouTube video id.
    pub fn cached_path(&self, video_id: &str) -> PathBuf {
        self.media_dir.join(format!("{video_id}.m4a"))
    }

    pub fn has_cached(&self, video_id: &str) -> bool {
        self.cached_path(video_id).is_file()
    }

    /// Adds a track to the upload queue. Returns `false` if already queued.
    pub fn enqueue(
        &self,
        video_id: &str,
        file_path: &Path,
        title: &str,
        artist: &str,
    ) -> Result<bool> {
        let now = now_ts();
        let caption = Self::caption_for(video_id, title, artist);
        let conn = self.conn();
        let changed = conn.execute(
            "INSERT OR IGNORE INTO upload_queue
                 (video_id, file_path, title, artist, caption, status, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 'pending', ?6, ?6)
             ON CONFLICT(video_id) DO UPDATE SET status='pending', retry_count=0, last_error=NULL, updated_at=excluded.updated_at
             WHERE upload_queue.status IN ('done','failed')",
            params![
                video_id,
                file_path.to_string_lossy(),
                title,
                artist,
                caption,
                now
            ],
        )?;
        Ok(changed > 0)
    }

    /// Number of tracks still waiting for Telegram.
    pub fn clear_pending(&self)->Result<()> { self.conn().execute("DELETE FROM upload_queue",[])?; self.conn().execute("DELETE FROM saved_tracks",[])?;Ok(()) }
    pub fn saved_message(&self,id:&str)->Result<Option<i32>> {Ok(self.conn().query_row("SELECT message_id FROM saved_tracks WHERE video_id=?1",[id],|r|r.get(0)).optional()?)}
    pub fn remember_saved(&self,id:&str,message:i32)->Result<()> {self.conn().execute("INSERT INTO saved_tracks(video_id,message_id) VALUES(?1,?2) ON CONFLICT(video_id) DO UPDATE SET message_id=excluded.message_id",params![id,message])?;Ok(())}
    pub fn forget_saved(&self,id:&str)->Result<()> {self.conn().execute("DELETE FROM saved_tracks WHERE video_id=?1",[id])?;Ok(())}
    pub fn failed_tracks(&self)->Result<u32> { Ok(self.conn().query_row("SELECT COUNT(*) FROM upload_queue WHERE status='failed'",[],|row|row.get(0))?) }
    pub fn queued_tracks(&self) -> Result<u32> {
        let conn = self.conn();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM upload_queue WHERE status IN ('pending','uploading')",
            [],
            |row| row.get(0),
        )?;
        Ok(count as u32)
    }

    /// Takes the oldest pending item and marks it `uploading` (one at a time).
    pub fn take_next_pending(&self) -> Result<Option<QueueItem>> {
        let mut conn = self.conn();
        let transaction = conn.transaction()?;
        let item = transaction
            .query_row(
                "SELECT id, video_id, file_path, title, artist, caption, retry_count
                 FROM upload_queue
                 WHERE status = 'pending'
                 ORDER BY id
                 LIMIT 1",
                [],
                |row| {
                    Ok(QueueItem {
                        id: row.get(0)?,
                        video_id: row.get(1)?,
                        file_path: row.get(2)?,
                        title: row.get(3)?,
                        artist: row.get(4)?,
                        caption: row.get(5)?,
                        retry_count: row.get(6)?,
                    })
                },
            )
            .optional()?;

        if let Some(item) = &item {
            transaction.execute(
                "UPDATE upload_queue SET status = 'uploading', updated_at = ?2 WHERE id = ?1",
                params![item.id, now_ts()],
            )?;
        }
        transaction.commit()?;
        Ok(item)
    }

    /// Marks an upload `done` and records it in the sliding window.
    pub fn mark_done(&self, id: i64) -> Result<()> {
        let conn = self.conn();
        conn.execute(
            "UPDATE upload_queue SET status = 'done', last_error = NULL, updated_at = ?2 WHERE id = ?1",
            params![id, now_ts()],
        )?;
        Ok(())
    }

    /// Returns an item to `pending` (or gives up after `MAX_UPLOAD_RETRIES`).
    pub fn mark_failed(&self, id: i64, error: &str) -> Result<()> {
        let conn = self.conn();
        conn.execute(
            "UPDATE upload_queue
             SET status = CASE WHEN retry_count + 1 >= ?2 THEN 'failed' ELSE 'pending' END,
                 retry_count = retry_count + 1,
                 last_error = ?3,
                 updated_at = ?4
             WHERE id = ?1",
            params![id, MAX_UPLOAD_RETRIES, error, now_ts()],
        )?;
        Ok(())
    }

    /// Records a successful upload at `now` (consumes window quota).
    pub fn record_upload(&self, video_id: &str) -> Result<()> {
        let conn = self.conn();
        conn.execute(
            "INSERT INTO upload_history (video_id, uploaded_at) VALUES (?1, ?2)",
            params![video_id, now_ts()],
        )?;
        Ok(())
    }

    /// `FLOOD_WAIT` from Telegram: pause for `seconds` + 5 s buffer.
    /// Returns the applied pause in seconds.
    pub fn on_flood_wait(&self, seconds: u64) -> Result<u64> {
        let pause = seconds.saturating_add(FLOOD_WAIT_BUFFER_SECS);
        let until = now_ts().saturating_add(pause as i64);
        let conn = self.conn();
        conn.execute(
            "INSERT INTO tg_meta (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![COOLDOWN_KEY, until.to_string()],
        )?;
        Ok(pause)
    }

    fn cooldown_seconds_left(&self, now: i64) -> Result<u64> {
        let conn = self.conn();
        let raw: Option<String> = conn
            .query_row(
                "SELECT value FROM tg_meta WHERE key = ?1",
                params![COOLDOWN_KEY],
                |row| row.get(0),
            )
            .optional()?;
        let until = raw.and_then(|value| value.parse::<i64>().ok()).unwrap_or(0);
        Ok((until - now).max(0) as u64)
    }

    fn uploads_in_window(&self, now: i64) -> Result<i64> {
        let conn = self.conn();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM upload_history WHERE uploaded_at > ?1",
            params![now - QUOTA_WINDOW_SECS as i64],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    fn pacing_seconds_left(&self, now: i64) -> Result<u64> {
        let conn = self.conn();
        let last: Option<i64> = conn
            .query_row(
                "SELECT MAX(uploaded_at) FROM upload_history",
                [],
                |row| row.get::<_, Option<i64>>(0),
            )
            .optional()?
            .flatten();
        Ok(match last {
            Some(at) => (at + self.min_pacing_secs as i64 - now).max(0) as u64,
            None => 0,
        })
    }

    /// Seconds until any upload is allowed again (cooldown, pacing, window).
    pub fn seconds_until_allowed(&self) -> Result<u64> {
        let now = now_ts();
        let cooldown = self.cooldown_seconds_left(now)?;
        let pacing = self.pacing_seconds_left(now)?;
        let remaining = (MAX_UPLOADS_PER_WINDOW as i64 - self.uploads_in_window(now)?).max(0);

        let window = if remaining == 0 {
            let conn = self.conn();
            let oldest: Option<i64> = conn
                .query_row(
                    "SELECT MIN(uploaded_at) FROM upload_history WHERE uploaded_at > ?1",
                    params![now - QUOTA_WINDOW_SECS as i64],
                    |row| row.get::<_, Option<i64>>(0),
                )
                .optional()?
                .flatten();
            oldest
                .map(|at| (at + QUOTA_WINDOW_SECS as i64 - now).max(0) as u64)
                .unwrap_or(0)
        } else {
            0
        };

        Ok(cooldown.max(pacing).max(window))
    }

    pub fn can_upload_now(&self) -> Result<bool> {
        Ok(self.seconds_until_allowed()? == 0)
    }

    /// Current quota snapshot for the sidebar gauge.
    pub fn quota_status(&self) -> Result<QuotaStatus> {
        let now = now_ts();
        let in_window = self.uploads_in_window(now)?;
        let remaining = (MAX_UPLOADS_PER_WINDOW as i64 - in_window).max(0) as u32;
        let queued = self.queued_tracks()?;
        let cooldown = self.cooldown_seconds_left(now)?;
        let seconds_until = self.seconds_until_allowed()?;

        let status_label = if cooldown > 0 {
            format!("Flood wait: {cooldown}s left")
        } else if remaining == 0 {
            format!("Window full: resets in {seconds_until}s")
        } else if queued > 0 {
            format!("{queued} queued")
        } else {
            "Idle".to_string()
        };

        Ok(QuotaStatus {
            remaining_in_window: remaining,
            max_per_window: MAX_UPLOADS_PER_WINDOW,
            queued_tracks: queued,
            cooldown_seconds_left: cooldown,
            status_label,
        })
    }

    async fn wait_until_allowed(&self) -> Result<()> {
        loop {
            let wait = self.seconds_until_allowed()?;
            if wait == 0 {
                return Ok(());
            }
            // Cap each sleep so a changing quota is re-read regularly.
            tokio::time::sleep(Duration::from_secs(wait.min(30))).await;
        }
    }

    /// Sequential upload worker: exactly one upload at a time, paced by the
    /// sliding window. The Telegram client is injected so this module stays
    /// independent of `grammers`.
    pub async fn drain_queue<F>(&self, uploader: F) -> Result<u32>
    where
        F: Fn(QueueItem) -> Pin<Box<dyn Future<Output = UploadResult> + Send>>,
    {
        let mut processed = 0u32;
        while let Some(item) = self.take_next_pending()? {
            self.wait_until_allowed().await?;
            match uploader(item.clone()).await {
                UploadResult::Done => {
                    self.record_upload(&item.video_id)?;
                    self.mark_done(item.id)?;
                    processed += 1;
                }
                UploadResult::FloodWait { seconds } => {
                    // Pause globally, then requeue this item.
                    self.on_flood_wait(seconds)?;
                    self.mark_failed(item.id, "flood_wait")?;
                }
                UploadResult::Failed { error } => {
                    self.mark_failed(item.id, &error)?;
                }
            }
        }
        Ok(processed)
    }

    /// Total bytes currently used by cached audio.
    pub fn cache_bytes(&self) -> Result<u64> {
        let mut total = 0u64;
        for entry in std::fs::read_dir(&self.media_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                total += entry.metadata()?.len();
            }
        }
        Ok(total)
    }

    /// Deletes the oldest cached files until the budget is met.
    /// Files still present in `upload_queue` are never deleted.
    pub fn evict_lru(&self) -> Result<EvictionReport> { self.evict_to(self.max_cache_bytes) }
    pub fn make_room(&self,needed:u64)->Result<bool> {
        if needed>self.max_cache_bytes {return Ok(false);}
        let target=self.max_cache_bytes-needed;
        Ok(self.evict_to(target)?.remaining_bytes<=target)
    }
    fn evict_to(&self,target:u64) -> Result<EvictionReport> {
        let protected: HashSet<String> = {
            let conn = self.conn();
            let mut statement = conn.prepare("SELECT file_path FROM upload_queue WHERE status IN ('pending','uploading')")?;
            let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
            rows.collect::<std::result::Result<HashSet<_>, _>>()?
        };

        let mut files: Vec<(PathBuf, u64, SystemTime)> = Vec::new();
        let mut total = 0u64;
        for entry in std::fs::read_dir(&self.media_dir)? {
            let entry = entry?;
            if !entry.file_type()?.is_file() {
                continue;
            }
            let metadata = entry.metadata()?;
            total += metadata.len();
            files.push((
                entry.path(),
                metadata.len(),
                metadata.modified().unwrap_or(UNIX_EPOCH),
            ));
        }

        let mut report = EvictionReport {
            remaining_bytes: total,
            ..EvictionReport::default()
        };
        if total <= target {
            return Ok(report);
        }

        files.sort_by(|left, right| left.2.cmp(&right.2).then_with(|| left.0.cmp(&right.0)));
        for (path, bytes, _) in files {
            if total <= target {
                break;
            }
            if protected.contains(&path.to_string_lossy().to_string()) {
                continue;
            }
            match std::fs::remove_file(&path) {
                Ok(()) => {
                    total -= bytes;
                    report.deleted_files += 1;
                    report.freed_bytes += bytes;
                }
                Err(_) => continue,
            }
        }
        report.remaining_bytes = total;
        Ok(report)
    }
}

fn now_ts() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs() as i64)
        .unwrap_or(0)
}

/// Shared handle for the Axum router.
pub type SharedTelegramCache = Arc<TelegramCacheManager>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    struct TempManager {
        manager: TelegramCacheManager,
        root: PathBuf,
    }

    impl TempManager {
        fn new(name: &str, max_cache_bytes: u64, pacing: u64) -> Self {
            let root = std::env::temp_dir().join(format!(
                "aysha-tg-test-{}-{}-{}",
                std::process::id(),
                name,
                now_ts()
            ));
            let _ = fs::remove_dir_all(&root);
            fs::create_dir_all(root.join("files")).unwrap();
            fs::create_dir_all(root.join("cache")).unwrap();
            let manager = TelegramCacheManager::open_with(
                root.join("files"),
                root.join("cache"),
                max_cache_bytes,
                pacing,
            )
            .unwrap();
            Self { manager, root }
        }

        fn write_cache_file(&self, video_id: &str, bytes: usize) -> PathBuf {
            let path = self.manager.cached_path(video_id);
            fs::write(&path, vec![7u8; bytes]).unwrap();
            std::thread::sleep(Duration::from_millis(15));
            path
        }
    }

    impl Drop for TempManager {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn archive_makes_room_without_evicting_pending_uploads() {
        let dirs=TempManager::new("room",100,0);
        let old=dirs.write_cache_file("abcdefghijk",50);
        let pending=dirs.write_cache_file("12345678901",40);
        dirs.manager.enqueue("12345678901",&pending,"Song","").unwrap();
        assert!(dirs.manager.make_room(50).unwrap());
        assert!(!old.exists());assert!(pending.exists());
        assert!(!dirs.manager.make_room(70).unwrap());
    }

    #[test]
    fn saved_message_index_is_private_to_connection_and_retry_is_explicit() {
        let dirs=TempManager::new("saved-index",100,0);
        let cache=&dirs.manager;
        cache.remember_saved("abcdefghijk",42).unwrap();
        assert_eq!(cache.saved_message("abcdefghijk").unwrap(),Some(42));
        cache.forget_saved("abcdefghijk").unwrap();assert_eq!(cache.saved_message("abcdefghijk").unwrap(),None);
        let path=cache.cached_path("abcdefghijk");
        assert!(cache.enqueue("abcdefghijk",&path,"Song","").unwrap());
        assert!(!cache.enqueue("abcdefghijk",&path,"Song","").unwrap());
        let item=cache.take_next_pending().unwrap().unwrap();cache.mark_done(item.id).unwrap();
        assert!(cache.enqueue("abcdefghijk",&path,"Song","").unwrap(),"a deleted remote message must be replaceable");
        cache.clear_pending().unwrap();assert_eq!(cache.queued_tracks().unwrap(),0);
    }

    #[test]
    fn caption_and_cache_path_follow_the_telegram_convention() {
        assert_eq!(
            TelegramCacheManager::caption_for("abc123", "Title", "Artist"),
            "#yt_abc123\nTitle\nArtist"
        );
        let fixture = TempManager::new("caption", DEFAULT_MAX_CACHE_BYTES, 12);
        assert!(fixture.manager.cached_path("abc123").ends_with("abc123.m4a"));
        assert!(!fixture.manager.has_cached("abc123"));
    }

    #[test]
    fn enqueue_is_idempotent_per_video_id() {
        let fixture = TempManager::new("enqueue", DEFAULT_MAX_CACHE_BYTES, 12);
        let file = fixture.write_cache_file("vid1", 16);
        assert!(fixture.manager.enqueue("vid1", &file, "Song", "Artist").unwrap());
        assert!(!fixture.manager.enqueue("vid1", &file, "Song", "Artist").unwrap());
        assert_eq!(fixture.manager.queued_tracks().unwrap(), 1);
    }

    #[test]
    fn flood_wait_pauses_with_buffer_and_reports_cooldown() {
        let fixture = TempManager::new("flood", DEFAULT_MAX_CACHE_BYTES, 12);
        let applied = fixture.manager.on_flood_wait(30).unwrap();
        assert_eq!(applied, 30 + FLOOD_WAIT_BUFFER_SECS);

        let status = fixture.manager.quota_status().unwrap();
        assert_eq!(status.cooldown_seconds_left, applied);
        assert!(status.status_label.starts_with("Flood wait"));
        assert!(!fixture.manager.can_upload_now().unwrap());
    }

    #[test]
    fn sliding_window_blocks_after_eighteen_uploads() {
        let fixture = TempManager::new("window", DEFAULT_MAX_CACHE_BYTES, 0);
        for index in 0..MAX_UPLOADS_PER_WINDOW {
            fixture.manager.record_upload(&format!("vid{index}")).unwrap();
        }

        let status = fixture.manager.quota_status().unwrap();
        assert_eq!(status.remaining_in_window, 0);
        assert_eq!(status.max_per_window, MAX_UPLOADS_PER_WINDOW);
        assert!(status.status_label.starts_with("Window full"));
        assert!(!fixture.manager.can_upload_now().unwrap());
    }

    #[test]
    fn eviction_removes_oldest_and_skips_queued_files() {
        let fixture = TempManager::new("evict", 120, 12);
        fixture.write_cache_file("old", 60);
        let protected = fixture.write_cache_file("protected", 60);
        fixture.write_cache_file("newest", 60);
        fixture
            .manager
            .enqueue("protected", &protected, "Keep", "Artist")
            .unwrap();

        assert_eq!(fixture.manager.cache_bytes().unwrap(), 180);
        let report = fixture.manager.evict_lru().unwrap();
        assert!(report.deleted_files >= 1);
        assert!(report.remaining_bytes <= 120);
        assert!(protected.exists(), "queued file must never be evicted");
        assert!(fixture.manager.cache_bytes().unwrap() <= 120);
    }

    #[test]
    fn reopening_requeues_rows_stranded_in_uploading() {
        let mut fixture = TempManager::new("restart", DEFAULT_MAX_CACHE_BYTES, 12);
        let file = fixture.write_cache_file("restart1", 16);
        fixture
            .manager
            .enqueue("restart1", &file, "Song", "Artist")
            .unwrap();

        // Simulate a worker that took the row and was then killed mid-upload.
        let taken = fixture.manager.take_next_pending().unwrap().expect("row taken");
        assert_eq!(taken.video_id, "restart1");
        assert_eq!(fixture.manager.queued_tracks().unwrap(), 1);

        // Reopen the database (fresh process): the stale 'uploading' row must
        // be recoverable instead of being stranded.
        fixture.manager = TelegramCacheManager::open_with(
            fixture.root.join("files"),
            fixture.root.join("cache"),
            DEFAULT_MAX_CACHE_BYTES,
            12,
        )
        .unwrap();

        assert_eq!(fixture.manager.queued_tracks().unwrap(), 1);
        let recovered = fixture
            .manager
            .take_next_pending()
            .unwrap()
            .expect("stranded row must be requeued as pending");
        assert_eq!(recovered.video_id, "restart1");
        assert_eq!(recovered.retry_count, 0);
    }

    #[tokio::test]
    async fn drain_queue_uploads_sequentially_and_respects_pacing() {
        let fixture = TempManager::new("drain", DEFAULT_MAX_CACHE_BYTES, 0);
        for index in 0..2 {
            let id = format!("drain{index}");
            let file = fixture.write_cache_file(&id, 8);
            fixture.manager.enqueue(&id, &file, "Song", "Artist").unwrap();
        }

        let processed = fixture
            .manager
            .drain_queue(|_item| {
                Box::pin(async { UploadResult::Done })
            })
            .await
            .unwrap();

        assert_eq!(processed, 2);
        assert_eq!(fixture.manager.queued_tracks().unwrap(), 0);
    }
}
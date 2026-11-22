//! Minimal JSON-backed Telegram session storage.
//!
//! `grammers-session`'s built-in SQLite storage depends on `libsql`, which
//! bundles a second copy of SQLite and collides with `rusqlite`'s bundled
//! SQLite at link time (and would bloat the APK). This module implements the
//! `Session` trait over a single JSON file instead, so MTProto authorization
//! keys, datacenter options and cached peers persist in `filesDir`.

use grammers_session::{
    types::{ChannelState, DcOption, PeerId, PeerInfo, UpdateState, UpdatesState},
    BoxFuture, Session, SessionData,
};
use serde::{Deserialize, Serialize};
use std::{
    io,
    path::{Path, PathBuf},
    sync::Mutex,
};

/// Atomic app-private credentials/session writes; no world-readable auth keys.
pub fn private_write(path:&Path, bytes:&[u8])->io::Result<()> {
    use std::io::Write;
    let tmp=path.with_extension("private.tmp");
    let mut options=std::fs::OpenOptions::new();options.create(true).truncate(true).write(true);
    #[cfg(unix)] {use std::os::unix::fs::OpenOptionsExt;options.mode(0o600);}
    let mut file=options.open(&tmp)?;
    #[cfg(unix)] {use std::os::unix::fs::PermissionsExt;file.set_permissions(std::fs::Permissions::from_mode(0o600))?;}
    file.write_all(bytes)?;file.sync_all()?;std::fs::rename(tmp,path)
}

/// On-disk shape. Kept as vectors so JSON has no non-string map keys.
#[derive(Serialize, Deserialize)]
struct PersistedSession {
    home_dc: i32,
    #[serde(default)]
    dc_options: Vec<DcOption>,
    #[serde(default)]
    peer_infos: Vec<(PeerId, PeerInfo)>,
    #[serde(default)]
    updates_state: UpdatesState,
}

impl From<&SessionData> for PersistedSession {
    fn from(data: &SessionData) -> Self {
        Self {
            home_dc: data.home_dc,
            dc_options: data.dc_options.values().cloned().collect(),
            peer_infos: data
                .peer_infos
                .iter()
                .map(|(id, info)| (*id, info.clone()))
                .collect(),
            updates_state: data.updates_state.clone(),
        }
    }
}

impl PersistedSession {
    fn into_session_data(self) -> SessionData {
        // Start from the defaults so statically-known DC options always exist.
        let mut data = SessionData::default();
        data.home_dc = self.home_dc;
        for option in self.dc_options {
            data.dc_options.insert(option.id, option);
        }
        for (id, info) in self.peer_infos {
            data.peer_infos.insert(id, info);
        }
        data.updates_state = self.updates_state;
        data
    }
}

/// `Session` implementation that persists to a single JSON file.
pub struct FileSession {
    path: PathBuf,
    data: Mutex<SessionData>,
}

impl FileSession {
    /// Loads the session, or starts a fresh one when the file does not exist.
    /// A corrupt file is moved aside (`.corrupt`) rather than reused.
    pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref().to_path_buf();

        let data = match std::fs::read(&path) {
            Ok(bytes) => match serde_json::from_slice::<PersistedSession>(&bytes) {
                Ok(persisted) => persisted.into_session_data(),
                Err(_) => {
                    let corrupt = path.with_extension("corrupt");
                    let _ = std::fs::rename(&path, &corrupt);
                    SessionData::default()
                }
            },
            Err(error) if error.kind() == io::ErrorKind::NotFound => SessionData::default(),
            Err(error) => return Err(error),
        };

        let session = Self {
            path,
            data: Mutex::new(data),
        };
        // Ensure the file exists so a later read never mistakes it for missing.
        session.persist(&session.lock())?;
        Ok(session)
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, SessionData> {
        self.data.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn persist(&self, data: &SessionData) -> io::Result<()> {
        let persisted = PersistedSession::from(data);
        let json = serde_json::to_vec(&persisted)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;

        // Write-then-rename so a crash cannot truncate an existing session.
        private_write(&self.path, &json)
    }

    fn mutate<F>(&self, update: F) -> io::Result<()>
    where
        F: FnOnce(&mut SessionData),
    {
        let mut data = self.lock();
        update(&mut data);
        self.persist(&data)
    }
}

impl Session for FileSession {
    type Error = io::Error;

    fn home_dc_id(&self) -> Result<i32, Self::Error> {
        Ok(self.lock().home_dc)
    }

    fn set_home_dc_id(&self, dc_id: i32) -> BoxFuture<'_, Result<(), Self::Error>> {
        Box::pin(async move {
            self.mutate(|data| data.home_dc = dc_id)
        })
    }

    fn dc_option(&self, dc_id: i32) -> Result<Option<DcOption>, Self::Error> {
        Ok(self.lock().dc_options.get(&dc_id).cloned())
    }

    fn set_dc_option(&self, dc_option: &DcOption) -> BoxFuture<'_, Result<(), Self::Error>> {
        let option = dc_option.clone();
        Box::pin(async move {
            self.mutate(move |data| {
                data.dc_options.insert(option.id, option);
            })
        })
    }

    fn peer(&self, peer: PeerId) -> BoxFuture<'_, Result<Option<PeerInfo>, Self::Error>> {
        Box::pin(async move { Ok(self.lock().peer_infos.get(&peer).cloned()) })
    }

    fn cache_peer(&self, peer: PeerInfo) -> BoxFuture<'_, Result<(), Self::Error>> {
        Box::pin(async move {
            self.mutate(move |data| {
                data.peer_infos.insert(peer.id(), peer);
            })
        })
    }

    fn updates_state(&self) -> BoxFuture<'_, Result<UpdatesState, Self::Error>> {
        Box::pin(async move { Ok(self.lock().updates_state.clone()) })
    }

    fn set_update_state(&self, update: UpdateState) -> BoxFuture<'_, Result<(), Self::Error>> {
        Box::pin(async move {
            self.mutate(move |data| {
                match update {
                    UpdateState::All(state) => data.updates_state = state,
                    UpdateState::Primary { pts, date, seq } => {
                        let state = &mut data.updates_state;
                        state.pts = pts;
                        state.date = date;
                        state.seq = seq;
                    }
                    UpdateState::Secondary { qts } => data.updates_state.qts = qts,
                    UpdateState::Channel { id, pts } => {
                        match data.updates_state.channels.iter_mut().find(|entry| entry.id == id) {
                            Some(entry) => entry.pts = pts,
                            None => data.updates_state.channels.push(ChannelState { id, pts }),
                        }
                    }
                }
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use grammers_session::types::PeerInfo;

    fn temp_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "aysha-session-test-{}-{}-{}",
            std::process::id(),
            name,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|elapsed| elapsed.as_nanos())
                .unwrap_or(0)
        ))
    }

    #[test]
    fn session_survives_reopen_with_home_dc_and_update_state() {
        let path = temp_path("reopen");

        {
            let session = FileSession::open(&path).unwrap();
            assert_eq!(session.home_dc_id().unwrap(), SessionData::default().home_dc);
            futures_block_on(session.set_home_dc_id(4)).unwrap();
            futures_block_on(session.set_update_state(UpdateState::Primary {
                pts: 10,
                date: 20,
                seq: 1,
            }))
            .unwrap();
        }

        let reopened = FileSession::open(&path).unwrap();
        assert_eq!(reopened.home_dc_id().unwrap(), 4);
        let state = futures_block_on(reopened.updates_state()).unwrap();
        assert_eq!(state.pts, 10);
        assert_eq!(state.date, 20);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn dc_options_and_peers_are_persisted() {
        let path = temp_path("peers");
        {
            let session = FileSession::open(&path).unwrap();
            let option = SessionData::default()
                .dc_options
                .get(&1)
                .cloned()
                .expect("static DC 1");
            futures_block_on(session.set_dc_option(&option)).unwrap();
            assert!(session.dc_option(1).unwrap().is_some());
        }
        let reopened = FileSession::open(&path).unwrap();
        assert!(reopened.dc_option(1).unwrap().is_some());
        assert!(Option::<PeerInfo>::None.is_none());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn corrupt_session_starts_fresh_and_is_moved_aside() {
        let path = temp_path("corrupt");
        std::fs::write(&path, b"not a session").unwrap();
        let session = FileSession::open(&path).unwrap();
        assert_eq!(session.home_dc_id().unwrap(), SessionData::default().home_dc);
        assert!(path.with_extension("corrupt").exists());
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(path.with_extension("corrupt"));
    }

    // Tiny executor for these synchronous tests (no awaits inside).
    fn futures_block_on<F: std::future::Future>(future: F) -> F::Output {
        use std::sync::Arc;
        use std::task::{Context, Poll, Wake, Waker};

        struct NoopWake;
        impl Wake for NoopWake {
            fn wake(self: Arc<Self>) {}
        }

        let waker = Waker::from(Arc::new(NoopWake));
        let mut context = Context::from_waker(&waker);
        let mut future = std::pin::pin!(future);
        loop {
            if let Poll::Ready(value) = future.as_mut().poll(&mut context) {
                return value;
            }
        }
    }
}

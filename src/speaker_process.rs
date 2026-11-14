//! Native ownership of the legacy speaker child while its audio backend is migrated.
//! No global pkill, PID rediscovery, shell control or shared IPC socket.
use std::{io, path::{Path, PathBuf}, sync::atomic::{AtomicU64, Ordering}};
use std::os::unix::{ffi::OsStrExt, fs::DirBuilderExt};
use tokio::{sync::oneshot, task::JoinHandle};

static DIRECTORY_SEQUENCE: AtomicU64 = AtomicU64::new(0);
pub struct PrivateIpc { directory: PathBuf, socket: PathBuf }
impl PrivateIpc {
    pub fn create() -> io::Result<Self> {
        for _ in 0..8 {
            let sequence = DIRECTORY_SEQUENCE.fetch_add(1,Ordering::Relaxed);
            let directory = std::env::temp_dir().join(format!("aysha-speaker-{}-{sequence}",std::process::id()));
            let socket = directory.join("control.sock");
            if socket.as_os_str().as_bytes().len() >= 108 {
                return Err(io::Error::new(io::ErrorKind::InvalidInput,"speaker socket path too long"));
            }
            match std::fs::DirBuilder::new().mode(0o700).create(&directory) {
                Ok(()) => return Ok(Self { directory, socket }),
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(e),
            }
        }
        Err(io::Error::new(io::ErrorKind::AlreadyExists,"speaker private directory unavailable"))
    }
    pub fn socket(&self) -> &Path { &self.socket }
}
impl Drop for PrivateIpc {
    fn drop(&mut self) {
        // Only two exact paths in the directory this object created. Never recurse,
        // remove a shared socket, follow names found on disk or touch another owner.
        let _ = std::fs::remove_file(&self.socket);
        let _ = std::fs::remove_dir(&self.directory);
    }
}

pub struct SpeakerProcess {
    cancel: Option<oneshot::Sender<()>>,
    task: Option<JoinHandle<io::Result<()>>>,
    socket: PathBuf,
}
impl SpeakerProcess {
    pub fn spawn(command: std::process::Command, ipc: PrivateIpc) -> io::Result<Self> {
        let socket = ipc.socket().to_path_buf();
        let mut command = tokio::process::Command::from(command);
        command.kill_on_drop(true);
        let mut child = command.spawn()?;
        let (cancel, receive) = oneshot::channel();
        let task = tokio::spawn(async move {
            let _owned_ipc = ipc;
            tokio::select! {
                result = child.wait() => { result?; }
                _ = receive => { child.kill().await?; }
            }
            Ok(())
        });
        Ok(Self { cancel: Some(cancel), task: Some(task), socket })
    }
    pub fn socket(&self) -> Option<&Path> {
        self.task.as_ref().filter(|task| !task.is_finished()).map(|_| self.socket.as_path())
    }
    pub async fn stop(mut self) -> Result<(), &'static str> {
        if let Some(cancel) = self.cancel.take() { let _ = cancel.send(()); }
        if let Some(mut task) = self.task.take() {
            match tokio::time::timeout(std::time::Duration::from_secs(3), &mut task).await {
                Ok(Ok(Ok(()))) => Ok(()),
                Ok(_) => Err("speaker_stop_failed"),
                Err(_) => {
                    task.abort();
                    // Aborting drops Child with kill_on_drop. Runtime performs
                    // best-effort reaping; the timeout is not a successful stop.
                    Err("speaker_stop_timeout")
                }
            }
        } else { Ok(()) }
    }
}
impl Drop for SpeakerProcess {
    fn drop(&mut self) {
        // A cancelled HTTP start must not strand a speaker process. The owner
        // task receives cancellation and kills/reaps only its own Child handle.
        if let Some(cancel) = self.cancel.take() { let _ = cancel.send(()); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    #[test] fn private_ipc_is_unique_private_and_does_not_remove_foreign_paths() {
        let first = PrivateIpc::create().unwrap(); let second = PrivateIpc::create().unwrap();
        assert_ne!(first.socket(),second.socket());
        assert_eq!(std::fs::metadata(&first.directory).unwrap().permissions().mode() & 0o777,0o700);
        let first_dir=first.directory.clone();let second_dir=second.directory.clone();
        drop(first);assert!(!first_dir.exists());assert!(second_dir.exists());
        drop(second);assert!(!second_dir.exists());
    }
    #[tokio::test] async fn owned_speaker_stop_kills_only_owned_child_and_reaps_it() {
        let mut other=tokio::process::Command::new("sleep").arg("30").kill_on_drop(true).spawn().unwrap();
        let ipc=PrivateIpc::create().unwrap();let directory=ipc.directory.clone();
        let mut command=std::process::Command::new("sleep"); command.arg("30");
        let owner=SpeakerProcess::spawn(command,ipc).unwrap();assert!(owner.socket().is_some());
        owner.stop().await.unwrap();
        assert!(!directory.exists());assert!(other.try_wait().unwrap().is_none());
        other.kill().await.unwrap();
    }
    #[tokio::test] async fn natural_exit_and_drop_release_owned_resources() {
        let ipc=PrivateIpc::create().unwrap();let directory=ipc.directory.clone();
        let owner=SpeakerProcess::spawn(std::process::Command::new("true"),ipc).unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(3),async {
            while owner.socket().is_some() { tokio::task::yield_now().await; }
        }).await.unwrap();
        owner.stop().await.unwrap();assert!(!directory.exists());
        let ipc=PrivateIpc::create().unwrap();let directory=ipc.directory.clone();
        let mut command=std::process::Command::new("sleep");command.arg("30");
        drop(SpeakerProcess::spawn(command,ipc).unwrap());
        tokio::time::timeout(std::time::Duration::from_secs(3),async {
            while directory.exists() { tokio::task::yield_now().await; }
        }).await.unwrap();
    }
    #[tokio::test] async fn failed_spawn_cleans_private_directory() {
        let ipc=PrivateIpc::create().unwrap();let directory=ipc.directory.clone();
        assert!(SpeakerProcess::spawn(std::process::Command::new("/no-such-aysha-test-program"),ipc).is_err());
        assert!(!directory.exists());
    }
}

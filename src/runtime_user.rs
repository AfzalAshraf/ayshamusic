//! Native Linux runtime-directory lookup. No `id` subprocess or extra dependency.
use std::{io::Read, path::PathBuf};

fn effective_uid(status: &str) -> Option<u32> {
    let mut rows = status.lines().filter_map(|line| line.strip_prefix("Uid:"));
    let row = rows.next()?;
    if rows.next().is_some() { return None; }
    let ids: Vec<_> = row.split_ascii_whitespace().collect();
    if ids.len() != 4 || ids.iter().any(|s| s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit())) { return None; }
    let numbers: Vec<_> = ids.iter().map(|s| s.parse::<u32>()).collect::<Result<_,_>>().ok()?;
    Some(numbers[1])
}
fn resolve(explicit: Option<String>, status: &str) -> Option<PathBuf> {
    if let Some(path) = explicit {
        let path = PathBuf::from(path);
        if path.is_absolute() { return Some(path); }
    }
    effective_uid(status).map(|uid| PathBuf::from(format!("/run/user/{uid}")))
}
pub fn runtime_directory() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("XDG_RUNTIME_DIR") {
        if PathBuf::from(&path).is_absolute() { return Some(path.into()); }
    }
    let mut status = String::new();
    std::fs::File::open("/proc/self/status").ok()?.take(16 * 1024).read_to_string(&mut status).ok()?;
    resolve(None, &status)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn native_uid_uses_effective_not_real_id() {
        assert_eq!(effective_uid("Name:\tserver\nUid:\t1000\t1001\t1002\t1003\n"),Some(1001));
        assert_eq!(resolve(None,"Uid: 1000 1001 1002 1003"),Some("/run/user/1001".into()));
    }
    #[test] fn native_uid_preserves_absolute_environment_and_rejects_malformed_ids() {
        assert_eq!(resolve(Some("/run/custom".into()),""),Some("/run/custom".into()));
        for text in ["", "Uid: 1 2", "Uid: 1 2 3 4 5", "Uid: 1 -2 3 4", "Uid: 1 999999999999 3 4", "Uid: 1 2 3 4\nUid: 1 2 3 4"] {
            assert!(resolve(Some("relative".into()),text).is_none());
        }
    }
}

//! Pure queue/history transitions, independent of browsers, mpv and Bluetooth.
use super::{PlayerState, Track};

impl PlayerState {
    pub(super) fn remember_current(&mut self, next: &Track) {
        if let Some(current) = &self.current_track {
            if current.url != next.url {
                self.history.push(current.clone());
                if self.history.len() > 200 { self.history.remove(0); }
            }
        }
    }

    pub(super) fn previous_track(&mut self) -> Option<Track> {
        let previous = self.history.pop()?;
        if let Some(current) = &self.current_track {
            if self.queue.first().map(|track| &track.url) != Some(&current.url) {
                self.queue.insert(0, current.clone());
                self.queue.truncate(200);
            }
        }
        Some(previous)
    }

    pub(super) fn enqueue_related(&mut self, tracks: Vec<Track>) {
        for track in tracks {
            if self.queue.len() >= 200 { break; }
            if self.current_track.as_ref().map(|t| &t.url) != Some(&track.url)
                && !self.queue.iter().any(|t| t.url == track.url) {
                self.queue.push(track);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn track(id: &str) -> Track {
        Track { id: id.into(), title: id.into(), artist: "test".into(), thumbnail: String::new(), url: format!("https://youtu.be/{id}") }
    }
    fn player() -> PlayerState {
        PlayerState { current_track: Some(track("C")), is_playing: true, volume: 0.8,
            status_msg: String::new(), history: vec![track("A"), track("B")],
            queue: vec![track("D")], liked: vec![], play_mode: "browser".into() }
    }
    #[test]
    fn previous_walks_back_instead_of_toggling_between_two_tracks() {
        let mut state = player();
        state.current_track = state.previous_track();
        assert_eq!(state.current_track.as_ref().unwrap().id, "B");
        state.current_track = state.previous_track();
        assert_eq!(state.current_track.as_ref().unwrap().id, "A");
        assert!(state.history.is_empty());
        assert_eq!(state.queue.iter().map(|t| t.id.as_str()).collect::<Vec<_>>(), vec!["B", "C", "D"]);
    }
    #[test]
    fn missing_history_preserves_current_and_queue() {
        let mut state = player();
        state.history.clear();
        assert!(state.previous_track().is_none());
        assert_eq!(state.current_track.unwrap().id, "C");
        assert_eq!(state.queue[0].id, "D");
    }
    #[test]
    fn replay_notification_does_not_duplicate_history() {
        let mut state = player();
        state.remember_current(&track("C"));
        assert_eq!(state.history.len(), 2);
        state.remember_current(&track("D"));
        assert_eq!(state.history.len(), 3);
    }
    #[test]
    fn related_tracks_do_not_replace_current_or_duplicate_queue() {
        let mut state = player();
        state.enqueue_related(vec![track("C"), track("D"), track("E"), track("E")]);
        assert_eq!(state.current_track.unwrap().id, "C");
        assert_eq!(state.queue.iter().map(|t| t.id.as_str()).collect::<Vec<_>>(), vec!["D", "E"]);
    }
}

//! Metadata resolution is not playback acceptance. Try an existing profile only
//! until both its source read and the selected decoder/output prepare succeed.
use crate::youtube::{ClientAttempt,InnertubeClient,ResolvedAudio};
use std::future::Future;

pub async fn select<F,Fut>(profiles:&[InnertubeClient],mut attempt:F)->Result<ResolvedAudio,Vec<ClientAttempt>>
where F:FnMut(InnertubeClient)->Fut,Fut:Future<Output=Result<ResolvedAudio,String>> {
    let mut failures=Vec::new();
    for profile in profiles {
        match attempt(*profile).await {
            Ok(audio)=>return Ok(audio),
            Err(error)=>{
                let terminal=error.contains("play_cancelled") || error=="source_prepare_deadline" || crate::youtube::terminal_provider_error(&error);
                failures.push(ClientAttempt{client:profile.name,status:None,visitor:"source_prepare",error});
                if terminal {break;}
            }
        }
    }
    Err(failures)
}

#[cfg(test)] mod tests {
    use super::*;
    use crate::youtube::{self,ANDROID,IOS};
    fn audio(profile:InnertubeClient)->ResolvedAudio {ResolvedAudio{video_id:"dQw4w9WgXcQ".into(),title:None,url:"https://r.googlevideo.com/audio".into(),itag:140,mime:"audio/mp4".into(),bitrate:128000,content_length:Some(1234),client:profile.name}}
    #[tokio::test] async fn rejected_media_not_merely_metadata_triggers_next_existing_profile(){
        let mut tried=Vec::new();
        let result=select(&youtube::CLIENTS,|profile| {tried.push(profile.name);async move {
            if profile.name==ANDROID.name {Err("media_range_http_403".into())}else{Ok(audio(profile))}
        }}).await.unwrap();
        assert_eq!(tried,vec![ANDROID.name,IOS.name]);assert_eq!(result.client,IOS.name);assert_eq!(result.user_agent(),IOS.user_agent);
    }
    #[tokio::test] async fn all_rejections_are_bounded_and_preserved_without_false_playing(){
        let mut calls=0;
        let result=select(&youtube::CLIENTS,|_|{calls+=1;async {Err("media_range_http_403".into())}}).await.unwrap_err();
        assert_eq!(calls,3);assert_eq!(result.len(),3);assert!(result.iter().all(|a|a.error.contains("403")));
    }
    #[tokio::test] async fn cancellation_deadline_and_forced_profile_do_not_probe_other_clients(){
        for error in ["play_cancelled","source_prepare_deadline"] {
            let mut calls=0;
            assert!(select(&youtube::CLIENTS,|_|{calls+=1;async move {Err(error.into())}}).await.is_err());assert_eq!(calls,1);
        }
        let mut calls=0;
        assert!(select(&[ANDROID],|_|{calls+=1;async {Err("403".into())}}).await.is_err());assert_eq!(calls,1);
    }
    #[tokio::test] async fn media3_source_failure_does_not_rotate_through_old_profiles() {
        let mut tried=Vec::new();
        let errors=select(youtube::playback_clients(true),|profile|{tried.push(profile.name);async {Err("media_prepare_tail_failed: media_range_http_403".into())}}).await.unwrap_err();
        assert_eq!(tried,vec![youtube::VISIONOS.name]);assert_eq!(errors.len(),1);
    }
    #[tokio::test] async fn sign_in_challenge_is_terminal_even_on_legacy_profile_list() {
        for error in ["LOGIN_REQUIRED","Sign in to confirm you're not a bot","provider_sign_in_or_bot_challenge"] {
            let mut calls=0;
            assert!(select(&youtube::CLIENTS,|_|{calls+=1;async move {Err(error.into())}}).await.is_err());
            assert_eq!(calls,1);
        }
    }

}

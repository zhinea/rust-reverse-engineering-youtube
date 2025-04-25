use std::sync::Arc;
use std::time::Duration;
use regex::Regex;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::sync::{broadcast, watch};
use tokio::task::JoinHandle;
use tokio::time::{interval, Interval};
use crate::utils::EventEmitter;

struct StreamChat {
    opts: StreamChatOptions,
    event_emitter: EventEmitter,
    is_initialized: Option<bool>,
    livechat_id: Option<String>,
    inteval_stop_tx: watch::Sender<bool>,
    interval_stop_handle: Option<JoinHandle<()>>
}

#[derive(Debug, Clone)]
pub struct StreamChatOptions {
    video_id: String,
    fetch_interval: u64
}

#[derive(Debug, Clone)]
pub struct StreamChatMetadata {
    api_key: String,
    client_version: String,
    continuation: String
}

impl StreamChat {

    pub fn new(opts: StreamChatOptions) -> Self {
        let event_emitter = EventEmitter::new(16);

        Self {
            event_emitter,
            is_initialized: Some(false),
            livechat_id: None,
            inteval_stop_tx: Default::default(),
            opts,
            interval_stop_handle: None,
        }
    }

    pub async fn connect(&mut self) {
        // if the chat is already connected, return
        if self.is_initialized.unwrap().eq(&true) {
            return;
        }

        let opts = self.fetch_metadata().await;
        let arc_opts = Arc::new(opts.unwrap());

        self.is_initialized = Some(true);
        let mut stop_tx = self.inteval_stop_tx.subscribe();

        let handle = tokio::spawn(async move {
            let mut ivl = interval(Duration::from_secs(3));
            ivl.tick().await;
            loop {
                tokio::select! {
                    _ = ivl.tick() => {
                        self.fech_chat(&arc_opts).await;
                    }
                    Ok(()) = stop_tx.changed()  => {
                        println!("Stopping streaming chat");
                        break;
                    }
                }
            }
        });

        self.interval_stop_handle = Some(handle);

    }



    async fn get_livechat(&self) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }


    async fn fetch_chat(&self, metadata: StreamChatMetadata) -> Result<(), Box<dyn std::error::Error>>{
        let body_request = serde_json::json!({
                "context": {
                    "client": {
                        "clientVersion": metadata.client_version,
                        "clientName": "WEB"
                    }
                },
                "continuation": metadata.continuation
            });

        let client = reqwest::Client::new();
        let res = client.post(
            format!("https://www.youtube.com/youtubei/v1/live_chat/get_live_chat?key={}", metadata.api_key)
        )
            .json(&body_request)
            .send()
            .await
            .expect("Error while getting chat!");

        println!("{:?}", res.text().await?);

        Ok(())
    }

    async fn fetch_metadata(&self) -> Result<StreamChatMetadata, Box<dyn std::error::Error>> {
        let live_id = self.opts.video_id.clone();
        let live_url = format!("https://www.youtube.com/watch?v={}", live_id);

        let html_page = reqwest::get(live_url)
            .await?
            .text()
            .await?;

        File::create("test.html").await?.write(&html_page.as_bytes()).await?;

        let re_canonical = Regex::new(
            r#"<link rel="canonical" href="https://www\.youtube\.com/watch\?v=(.+?)">"#
        ).unwrap();

        if !re_canonical.is_match(&html_page) {
            return Err("Video or Stream not found".into());
        }

        let re_has_chat_render = Regex::new(
            r#"liveChatRenderer"#
        ).unwrap();

        if !re_has_chat_render.is_match(&html_page) {
            return Err("Chat box not found".into());
        }

        let re_api_key = Regex::new(
            r#"INNERTUBE_API_KEY":"(.+?)"#
        ).unwrap();

        let api_key = re_api_key.find(&html_page).unwrap().as_str();

        if api_key.is_empty() {
            return Err("API Key not found".into());
        }

        let re_client_version = Regex::new(
            r#"clientVersion":"(.+?)"#
        ).unwrap();

        let client_version = re_client_version.find(&html_page).unwrap().as_str();

        if client_version.is_empty() {
            return Err("Client Version not found".into());
        }

        let re_continuation = Regex::new(
            r#"continuation":"(.+?)"#
        ).unwrap();

        let continuation = re_continuation.find(&html_page).unwrap().as_str();

        if continuation.is_empty() {
            return Err("Continuation not found".into());
        }

        println!("stream: {}\napi_key: {}\nclient_version: {}\ncontinuation: {}", live_id, api_key, client_version, continuation);

        Ok(StreamChatMetadata {
            api_key: api_key.to_string(),
            client_version: client_version.to_string(),
            continuation: continuation.to_string()
        })
    }
}

#[cfg(test)]
mod test {

    #[tokio::test]
    async fn test_fetch_metadata(){
        let chat = super::StreamChat::new(super::StreamChatOptions {
            video_id: "WxKil88rLJ4".to_string(),
            fetch_interval: 1000
        });

        let metadata = chat.fetch_metadata().await.expect("Error fetching metadata");


        println!("{:?}", metadata);
    }
}
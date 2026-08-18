use crate::{sync, turso};
use std::{path::PathBuf, sync::Arc, time::Duration};
use tokio::time::sleep;

pub fn start(path: PathBuf) {
    tauri::async_runtime::spawn(async move {
        loop {
            if let Ok(status) = sync::status(&path) {
                if status.pending > 0 {
                    if let Ok((url, token)) = turso::credentials() {
                        if !url.is_empty() && !token.is_empty() {
                            let _ = sync::sync_once(&path, url, token).await;
                        }
                    }
                }
            }
            sleep(Duration::from_secs(30)).await;
        }
    });
}

use bytes::Bytes;
use futures_util::Stream;
use moka::future::Cache;
use std::{
    io::{Seek, Write},
    mem,
    os::unix::fs::FileExt,
    sync::Arc,
    thread::JoinHandle,
    time::SystemTime,
};
use tokio::{
    sync::mpsc::{Receiver, Sender, channel},
    task::spawn_blocking,
};
use uuid::Uuid;

use crate::{ApiError, AppState, CacheEntry};

pub struct CacheFileStream {
    thread: Option<JoinHandle<()>>,
    recv: Receiver<Vec<u8>>,
}

impl CacheFileStream {
    pub fn new(file: Arc<std::fs::File>) -> Self {
        let (sender, recv) = channel(1);
        let thread = Some(std::thread::spawn(|| {
            Self::thread(file, sender);
        }));
        Self { thread, recv }
    }
    fn thread(file: Arc<std::fs::File>, sender: Sender<Vec<u8>>) {
        let mut buf = vec![0u8; 1024 * 1024];
        let mut offset = 0;
        loop {
            let read = file.read_at(&mut buf, offset).unwrap();
            if read == 0 {
                break;
            }
            let _ = sender.blocking_send(buf[..read].to_vec());
            offset += read as u64;
        }
    }
}

impl Stream for CacheFileStream {
    type Item = Result<bytes::Bytes, tokio::io::Error>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cv: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.recv
            .poll_recv(cv)
            .map(|buf| Ok(buf.map(Bytes::from)).transpose())
    }
}

impl Drop for CacheFileStream {
    fn drop(&mut self) {
        let _ = mem::take(&mut self.thread).map(JoinHandle::join);
    }
}

pub async fn cache_object(
    state: Arc<AppState>,
    cache: &Cache<Uuid, CacheEntry>,
    object_id: Uuid,
    enum_str: &str,
) -> Result<(), ApiError> {
    let mut object = state
        .s3_client
        .get_object()
        .bucket(enum_str.to_owned())
        .key(object_id.to_string())
        .send()
        .await?;
    let mut file = tempfile::tempfile_in("/cache").unwrap();
    while let Some(chunk) = object.body.try_next().await.unwrap() {
        file = spawn_blocking(move || {
            file.write_all(&chunk).unwrap();
            file
        })
        .await
        .unwrap();
    }
    file.flush().unwrap();
    cache
        .insert(
            object_id,
            CacheEntry {
                cached_at: SystemTime::now(),
                size_kb: file.stream_position().unwrap() as u32 / 1024,
                file: Arc::new(file),
            },
        )
        .await;
    Ok(())
}

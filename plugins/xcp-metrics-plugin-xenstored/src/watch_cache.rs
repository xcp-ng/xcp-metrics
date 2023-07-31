use dashmap::DashMap;
use futures::StreamExt;
use std::sync::Arc;
use tokio::{
    select,
    sync::mpsc::{self, error::SendError},
    task::{self, JoinHandle},
};
use xenstore_rs::{XBTransaction, Xs, XsOpenFlags};

/// A Xs watch cache that passively update values.
pub struct WatchCache {
    pub cache: Arc<DashMap<String, String>>,
    pub watch_task: JoinHandle<()>,
    watch_channel: mpsc::UnboundedSender<String>,
    unwatch_channel: mpsc::UnboundedSender<String>,
}

async fn watch_task(
    cache: Arc<DashMap<String, String>>,
    mut watch_channel: mpsc::UnboundedReceiver<String>,
    mut unwatch_channel: mpsc::UnboundedReceiver<String>,
) {
    let watch_task = task::spawn((|| {
        let cache = cache.clone();

        async move {
            let xs = Xs::new(XsOpenFlags::ReadOnly).unwrap();

            let mut stream = xs.get_stream().unwrap();

            while let Some(entry) = stream.next().await {
                match xs.read(XBTransaction::Null, &entry.path) {
                    Ok(value) => {
                        println!("Readed {} = {value}", entry.path);
                        cache.insert(entry.path, value);
                    }
                    Err(e) => {
                        println!("Removed {} ({e})", entry.path);
                        cache.remove(&entry.path);
                    }
                }
            }
        }
    })());

    let watch_channel_task = task::spawn(async move {
        let xs = Xs::new(XsOpenFlags::ReadOnly).unwrap();

        while let Some(to_watch) = watch_channel.recv().await {
            println!("Watching {to_watch}");
            xs.watch(&to_watch, "xcp-metrics-xenstored").ok();
        }
    });

    let unwatch_channel_task = task::spawn(async move {
        let xs = Xs::new(XsOpenFlags::ReadOnly).unwrap();

        while let Some(to_unwatch) = unwatch_channel.recv().await {
            println!("Unwatching {to_unwatch}");
            xs.unwatch(&to_unwatch, "xcp-metrics-xenstored").ok();
            cache.remove(&to_unwatch);
        }
    });

    select! {
        _ = watch_task => {},
        _ = watch_channel_task => {},
        _ = unwatch_channel_task => {},
    };
}

impl WatchCache {
    pub fn new() -> Self {
        let cache = Arc::new(DashMap::new());
        let (watch_sender, watch_receiver) = mpsc::unbounded_channel();
        let (unwatch_sender, unwatch_receiver) = mpsc::unbounded_channel();

        let watch_cache = cache.clone();
        let watch_task =
            task::spawn(
                async move { watch_task(watch_cache, watch_receiver, unwatch_receiver).await },
            );

        Self {
            cache,
            watch_task,
            watch_channel: watch_sender,
            unwatch_channel: unwatch_sender,
        }
    }

    pub fn watch(&self, path: &str) -> Result<(), SendError<String>> {
        self.watch_channel.send(path.to_string())
    }

    pub fn unwatch(&self, path: &str) -> Result<(), SendError<String>> {
        self.unwatch_channel.send(path.to_string())
    }

    pub fn read(&self, path: &str) -> Option<String> {
        self.cache.get(path).map(|value| value.clone())
    }
}

impl Default for WatchCache {
    fn default() -> Self {
        Self::new()
    }
}

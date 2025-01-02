//! # File Update Monitor
//!
//! `file_update_monitor` is a library for monitoring file changes. It provides a simple interface to watch
//! file changes in directories and execute custom callback functions when file content changes.
//!
//! ## Main Features
//!
//! - Monitor file changes in specified directories and subdirectories
//! - Support debouncing to avoid too frequent updates
//! - Asynchronous handling of file change events
//! - Customizable logic for handling file changes
//!
//! ## Example
//!
//! ```rust
//! use file_update_monitor::Monitor;
//! use std::error::Error;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn Error>> {
//!     // 创建一个监控器实例，监控当前目录，更新间隔为1秒
//!     let monitor = Monitor::new("./", 1000, |path| {
//!         println!("检测到文件变化: {}", path);
//!         Ok(())
//!     });
//!     
//!     // 启动监控
//!     monitor.start().await;
//!     Ok(())
//! }
//! ```

use debounce::EventDebouncer;
use futures::{
    channel::mpsc::{channel, Receiver},
    SinkExt, StreamExt,
};
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::error;
use std::{path::Path, sync::Arc, time::Duration};

/// Generic result type for error handling
type Result<T> = std::result::Result<T, Box<dyn error::Error>>;

/// Main struct for file monitoring
pub struct Monitor {
    /// Directory path to monitor
    dir: String,
    /// Update interval in milliseconds
    update_interval: u64,
    /// Callback function for file changes
    on_change: Arc<Box<dyn Fn(String) -> Result<()> + Send + Sync>>,
}

impl Monitor {
    /// Create a new monitor instance
    ///
    /// # Arguments
    ///
    /// * `dir` - Directory path to monitor
    /// * `update_interval` - Update interval in milliseconds
    /// * `on_change` - Callback function called when files change
    ///
    /// # Example
    ///
    /// ```
    /// use file_update_monitor::Monitor;
    ///
    /// let monitor = Monitor::new("./", 1000, |path| {
    ///     println!("File changed: {}", path);
    ///     Ok(())
    /// });
    /// ```
    pub fn new<F>(dir: &str, update_interval: u64, on_change: F) -> Self
    where
        F: Fn(String) -> Result<()> + Send + Sync + 'static,
    {
        Self {
            dir: dir.to_string(),
            update_interval,
            on_change: Arc::new(Box::new(on_change)),
        }
    }

    /// Start file monitoring
    ///
    /// This is an async method that will continue monitoring the specified directory until program termination
    pub async fn start(&self) {
        if let Err(e) = self.watch_directory().await {
            eprintln!("File monitoring error: {:?}", e);
        }
    }

    /// Internal method: implements core directory monitoring logic
    async fn watch_directory(&self) -> Result<()> {
        let delay = Duration::from_millis(self.update_interval);
        let on_change = self.on_change.clone();
        let debouncer = EventDebouncer::new(delay, move |path: String| on_change(path).unwrap());

        let (mut watcher, mut rx) = self.create_watcher()?;
        watcher.watch(Path::new(&self.dir), RecursiveMode::Recursive)?;

        while let Some(res) = rx.next().await {
            match res {
                Ok(event) => {
                    if let Some(path) = self.get_valid_path(event) {
                        debouncer.put(path);
                    }
                }
                Err(e) => eprintln!("File monitoring error: {:?}", e),
            }
        }

        Ok(())
    }

    /// Create filesystem event watcher
    fn create_watcher(
        &self,
    ) -> notify::Result<(RecommendedWatcher, Receiver<notify::Result<Event>>)> {
        let (mut tx, rx) = channel(1);

        let watcher = RecommendedWatcher::new(
            move |res| {
                futures::executor::block_on(async {
                    if let Err(e) = tx.send(res).await {
                        eprintln!("Error sending event: {:?}", e);
                    }
                })
            },
            Config::default(),
        )?;

        Ok((watcher, rx))
    }

    /// Extract valid file path from filesystem event
    ///
    /// Only processes file content modification events, ignores other types of events
    fn get_valid_path(&self, event: Event) -> Option<String> {
        if !matches!(
            event.kind,
            notify::EventKind::Modify(notify::event::ModifyKind::Data(
                notify::event::DataChange::Content
            ))
        ) {
            return None;
        }

        event
            .paths
            .first()
            .map(|path| path.to_str().unwrap().to_string())
    }
}

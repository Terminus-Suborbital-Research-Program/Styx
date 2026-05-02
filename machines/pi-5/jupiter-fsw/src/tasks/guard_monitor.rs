use notify::{Watcher, RecursiveMode, EventKind, event::ModifyKind};
use std::path::Path;
use std::sync::mpsc::{channel, Receiver};
use std::time::{Duration, Instant};
use std::fs::create_dir_all;

use crate::data::status::ExperimentColorState;

#[cfg(feature = "packet_logging")]
use log::info;

pub struct GuardMonitor {
    notify_rx: Receiver<notify::Result<notify::Event>>,
    _watcher: notify::RecommendedWatcher,
    check_interval: Duration,
    last_check: Instant,
}

impl GuardMonitor {
  
    pub fn new(path: &str, check_interval_secs: u64) -> Self {
        let (tx, rx) = channel();
        let mut watcher = notify::recommended_watcher(tx)
            .expect("Failed to create Guard watcher");

        let watch_path = Path::new(path);
        create_dir_all(watch_path).ok(); 

        watcher.watch(watch_path, RecursiveMode::NonRecursive)
            .expect("Failed to watch GUARD directory");

        Self {
            notify_rx: rx,
            _watcher: watcher,
            check_interval: Duration::from_secs(check_interval_secs),
            last_check: Instant::now(),
        }
    }

    pub fn update(&mut self, color_status: &mut ExperimentColorState) {
        let now = Instant::now();
        
        // Only perform the drain if the intialized check interval has passed
        if now.duration_since(self.last_check) >= self.check_interval {
            let mut guard_was_active = false;

            // Drain the backlog of filesystem events instantly
            while let Ok(event_res) = self.notify_rx.try_recv() {
                if let Ok(event) = event_res {
                    match event.kind {
                        EventKind::Modify(ModifyKind::Data(_)) | EventKind::Create(_) => {
                            guard_was_active = true;
                        }
                        _ => {}
                    }
                }
            }

            // If a file update is spotted, watchdog is updated
            if guard_was_active {
                color_status.feed_geiger();
                
                #[cfg(feature = "packet_logging")]
                info!("Guard data actively writing over the last {} seconds.", self.check_interval.as_secs());
            }

            // Reset for new check interval
            self.last_check = now;
        }
    }
}
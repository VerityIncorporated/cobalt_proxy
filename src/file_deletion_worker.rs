use std::{collections::VecDeque, time::Duration};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::interval;
use chrono::{Utc, DateTime};

pub type FileDeletionQueue = Arc<Mutex<VecDeque<(String, DateTime<Utc>)>>>;

pub fn initialize_file_deletion_worker() -> FileDeletionQueue {
    let queue = Arc::new(Mutex::new(VecDeque::new()));

    let worker_queue = queue.clone();
    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(60));

        loop {
            interval.tick().await;

            let mut queue = worker_queue.lock().await;
            let now = Utc::now();

            queue.retain(|(file_path, delete_at)| {
                if *delete_at <= now {
                    if let Err(err) = std::fs::remove_file(file_path) {
                        eprintln!("Failed to delete file {}: {}", file_path, err);
                    } else {
                        println!("Filed deleted! {}", file_path);
                    }
                    false
                } else {
                    true
                }
            });
        }
    });

    queue
}

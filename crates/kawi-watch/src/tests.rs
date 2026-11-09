use std::fs;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use super::*;
use crate::clock::Clock;

const EVENT_TIMEOUT: Duration = Duration::from_secs(5);

struct ManualClock {
    now: Mutex<Instant>,
}

impl ManualClock {
    fn new(now: Instant) -> Self {
        Self {
            now: Mutex::new(now),
        }
    }

    fn advance(&self, duration: Duration) {
        let mut now = self.now.lock().unwrap_or_else(|error| error.into_inner());
        *now += duration;
    }
}

impl Clock for ManualClock {
    fn now(&self) -> Instant {
        *self.now.lock().unwrap_or_else(|error| error.into_inner())
    }
}

fn temp_dir(name: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(1);
    let dir = std::env::temp_dir().join(format!(
        "kawi_watch_{}_{}_{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed),
        name
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("directory");
    dir
}

#[test]
fn fingerprints_match_identical_content() {
    let dir = temp_dir("fingerprint");
    let path = dir.join("a.txt");
    fs::write(&path, "alpha\n").expect("write");
    let first = Fingerprint::read(&path).expect("first");
    fs::write(&path, "alpha\n").expect("rewrite");
    let second = Fingerprint::read(&path).expect("second");
    assert!(first.matches(&second));
    assert!(first.modified().is_some());
    fs::write(&path, "beta\n").expect("change");
    let third = Fingerprint::read(&path).expect("third");
    assert!(!first.matches(&third));
    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn debounce_waits_for_a_quiet_period() {
    let start = Instant::now();
    let mut debounce = Debounce::new(Duration::from_millis(75));
    debounce.push(vec![PathBuf::from("a")], start);
    assert!(debounce
        .take_ready(start + Duration::from_millis(74))
        .is_none());
    debounce.push(vec![PathBuf::from("b")], start + Duration::from_millis(50));
    assert!(debounce
        .take_ready(start + Duration::from_millis(124))
        .is_none());
    assert_eq!(
        debounce.take_ready(start + Duration::from_millis(125)),
        Some(vec![PathBuf::from("a"), PathBuf::from("b")])
    );
}

#[test]
fn watcher_reports_writes_and_renames() {
    let dir = temp_dir("events");
    let path = dir.join("a.txt");
    let temp = dir.join("a.tmp");
    fs::write(&path, "alpha\n").expect("write");
    let clock = Arc::new(ManualClock::new(Instant::now()));
    let (wake, _wake_rx) = mpsc::channel();
    let options = WatchOptions::with_clock(
        Duration::from_millis(25),
        Duration::from_millis(1),
        clock.clone(),
    );
    let service = WatchService::with_options(wake, options).expect("service");
    service.watch_directories_and_wait([dir.as_path()]);
    fs::write(&temp, "beta\n").expect("temp");
    fs::rename(&temp, &path).expect("rename");
    let deadline = Instant::now() + EVENT_TIMEOUT;
    let mut seen = false;
    while Instant::now() < deadline {
        if service.try_recv_paths().iter().any(|event| event == &path) {
            seen = true;
            break;
        }
        clock.advance(Duration::from_millis(25));
        thread::yield_now();
    }
    assert!(seen, "rename target was not reported");
    drop(service);
    fs::remove_dir_all(dir).expect("cleanup");
}

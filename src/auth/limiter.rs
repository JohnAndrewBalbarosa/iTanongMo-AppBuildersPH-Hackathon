use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

const FAILURE_WINDOW: Duration = Duration::from_secs(60);
const MAX_FAILURES: usize = 5;
const MAX_TRACKED_EMAILS: usize = 4_096;

#[derive(Clone, Default)]
pub struct LoginLimiter {
    failures: Arc<Mutex<HashMap<String, VecDeque<Instant>>>>,
}

impl LoginLimiter {
    pub fn is_limited(&self, email: &str) -> bool {
        let now = Instant::now();
        let mut failures = self.lock();
        prune(&mut failures, now);
        failures
            .get(email)
            .is_some_and(|items| items.len() >= MAX_FAILURES)
    }

    pub fn record_failure(&self, email: String) {
        let now = Instant::now();
        let mut failures = self.lock();
        prune(&mut failures, now);
        if failures.len() >= MAX_TRACKED_EMAILS
            && !failures.contains_key(&email)
            && let Some(oldest_key) = failures
                .iter()
                .min_by_key(|(_, attempts)| attempts.front().copied())
                .map(|(key, _)| key.clone())
        {
            failures.remove(&oldest_key);
        }
        failures.entry(email).or_default().push_back(now);
    }

    pub fn clear(&self, email: &str) {
        self.lock().remove(email);
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<String, VecDeque<Instant>>> {
        self.failures
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

fn prune(failures: &mut HashMap<String, VecDeque<Instant>>, now: Instant) {
    failures.retain(|_, attempts| {
        while attempts
            .front()
            .is_some_and(|attempt| now.duration_since(*attempt) >= FAILURE_WINDOW)
        {
            attempts.pop_front();
        }
        !attempts.is_empty()
    });
}

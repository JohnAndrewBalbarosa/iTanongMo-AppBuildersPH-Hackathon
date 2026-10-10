use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use tokio::sync::Semaphore;

use crate::{
    ai::AiProvider,
    error::{AppError, AppResult},
};

use super::schemas::{RequesterAiDraft, RequesterAiInput};

#[derive(Clone)]
pub struct RequesterAiService {
    limits: Arc<Mutex<UsageLimits>>,
    in_flight: Arc<Semaphore>,
}

impl RequesterAiService {
    pub fn new() -> AppResult<Self> {
        Ok(Self {
            limits: Arc::new(Mutex::new(UsageLimits::new())),
            in_flight: Arc::new(Semaphore::new(2)),
        })
    }

    // Mental model: validate, reserve bounded usage, generate once, then validate the draft.
    pub async fn draft(
        &self,
        user_id: &str,
        provider: &dyn AiProvider,
        input: RequesterAiInput,
    ) -> AppResult<RequesterAiDraft> {
        let started = Instant::now();
        let correlation_id = uuid::Uuid::new_v4().to_string();
        let result = self.generate_draft(user_id, provider, input).await;
        match &result {
            Ok(_) => {
                tracing::info!(event="requester_ai.draft_completed", component="requester_ai", operation="draft", outcome="success", %correlation_id, duration_ms=started.elapsed().as_millis())
            }
            Err(error) => {
                tracing::warn!(event="requester_ai.draft_failed", component="requester_ai", operation="draft", outcome="rejected", %correlation_id, reason=%error, duration_ms=started.elapsed().as_millis())
            }
        }
        result
    }

    pub async fn draft_guest(
        &self,
        provider: &dyn AiProvider,
        input: RequesterAiInput,
    ) -> AppResult<RequesterAiDraft> {
        self.draft("__guest__", provider, input).await
    }

    async fn generate_draft(
        &self,
        user_id: &str,
        provider: &dyn AiProvider,
        input: RequesterAiInput,
    ) -> AppResult<RequesterAiDraft> {
        input.validate()?;
        let _permit = self
            .in_flight
            .try_acquire()
            .map_err(|_| AppError::RateLimited)?;
        self.limits
            .lock()
            .map_err(|_| AppError::Internal)?
            .reserve(user_id, Instant::now())?;
        let output = tokio::time::timeout(
            Duration::from_secs(45),
            provider.generate_json(
                "requester_draft",
                serde_json::to_value(input).map_err(|_| AppError::Internal)?,
            ),
        )
        .await
        .map_err(|_| AppError::AiUnavailable)??;
        serde_json::from_value::<RequesterAiDraft>(output)
            .map_err(|_| AppError::Validation("Invalid citizen AI draft JSON".into()))?
            .validate()
    }
}

struct UsageLimits {
    users: HashMap<String, UsageWindow>,
    shared: UsageWindow,
    guest_hourly: UsageWindow,
}

impl UsageLimits {
    fn new() -> Self {
        Self {
            users: HashMap::new(),
            shared: UsageWindow::new(Instant::now()),
            guest_hourly: UsageWindow::new(Instant::now()),
        }
    }

    fn reserve(&mut self, user_id: &str, now: Instant) -> AppResult<()> {
        self.users.retain(|_, window| !window.expired(now, 60));
        if self.shared.calls == 0 || self.shared.expired(now, 3600) {
            self.shared = UsageWindow::new(now);
        }
        if self.shared.calls >= 60
            || (!self.users.contains_key(user_id) && self.users.len() >= 1024)
        {
            return Err(AppError::RateLimited);
        }
        if user_id == "__guest__" {
            if self.guest_hourly.calls == 0 || self.guest_hourly.expired(now, 3600) {
                self.guest_hourly = UsageWindow::new(now);
            }
            if self.guest_hourly.calls >= 12 {
                return Err(AppError::RateLimited);
            }
        }
        let user = self
            .users
            .entry(user_id.to_owned())
            .or_insert_with(|| UsageWindow::new(now));
        if user.calls >= 5 {
            return Err(AppError::RateLimited);
        }
        user.calls += 1;
        self.shared.calls += 1;
        if user_id == "__guest__" {
            self.guest_hourly.calls += 1;
        }
        Ok(())
    }
}

struct UsageWindow {
    started: Instant,
    calls: u32,
}

impl UsageWindow {
    fn new(started: Instant) -> Self {
        Self { started, calls: 0 }
    }

    fn expired(&self, now: Instant, seconds: u64) -> bool {
        now.duration_since(self.started) >= Duration::from_secs(seconds)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct UnavailableProvider;

    #[async_trait::async_trait]
    impl AiProvider for UnavailableProvider {
        fn model_id(&self) -> &str {
            "unavailable-test"
        }

        async fn generate_json(
            &self,
            _: &str,
            _: serde_json::Value,
        ) -> AppResult<serde_json::Value> {
            Err(AppError::AiUnavailable)
        }
    }

    fn input() -> RequesterAiInput {
        RequesterAiInput {
            what_to_ask: "Please provide the latest public program reports.".into(),
            manual_title: "Public program reports".into(),
        }
    }

    #[tokio::test]
    async fn concurrency_rejection_does_not_spend_quota_and_failures_release_permits() {
        let service = RequesterAiService::new().unwrap();
        let permits = service.in_flight.acquire_many(2).await.unwrap();
        assert!(matches!(
            service
                .draft("citizen", &UnavailableProvider, input())
                .await,
            Err(AppError::RateLimited)
        ));
        assert_eq!(service.limits.lock().unwrap().shared.calls, 0);
        drop(permits);
        assert!(matches!(
            service
                .draft("citizen", &UnavailableProvider, input())
                .await,
            Err(AppError::AiUnavailable)
        ));
        assert_eq!(service.in_flight.available_permits(), 2);
        assert_eq!(service.limits.lock().unwrap().shared.calls, 1);
    }

    #[test]
    fn limits_count_failed_attempts_and_reset_expired_windows() {
        let now = Instant::now();
        let mut limits = UsageLimits::new();
        for _ in 0..5 {
            limits.reserve("citizen", now).unwrap();
        }
        assert!(matches!(
            limits.reserve("citizen", now),
            Err(AppError::RateLimited)
        ));
        limits
            .reserve("citizen", now + Duration::from_secs(60))
            .unwrap();
        for user in 0..54 {
            limits
                .reserve(&format!("other-{user}"), now + Duration::from_secs(60))
                .unwrap();
        }
        assert!(matches!(
            limits.reserve("another", now + Duration::from_secs(60)),
            Err(AppError::RateLimited)
        ));
        limits
            .reserve("citizen", now + Duration::from_secs(3600))
            .unwrap();
    }

    #[test]
    fn guest_limit_is_shared_across_an_hour() {
        let now = Instant::now();
        let mut limits = UsageLimits::new();
        for call in 0..12 {
            limits
                .reserve("__guest__", now + Duration::from_secs(call * 61))
                .unwrap();
        }
        assert!(matches!(
            limits.reserve("__guest__", now + Duration::from_secs(12 * 61)),
            Err(AppError::RateLimited)
        ));
        limits
            .reserve("__guest__", now + Duration::from_secs(3600))
            .unwrap();
    }
}

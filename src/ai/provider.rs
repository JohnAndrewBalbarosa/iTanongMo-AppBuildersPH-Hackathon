use async_trait::async_trait;
use serde_json::Value;

use crate::error::AppResult;

#[async_trait]
pub trait AiProvider: Send + Sync {
    fn model_id(&self) -> &str;
    async fn generate_json(&self, task_kind: &str, payload: Value) -> AppResult<Value>;
}

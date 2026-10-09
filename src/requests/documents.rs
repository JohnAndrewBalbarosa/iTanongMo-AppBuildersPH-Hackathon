use std::path::Path;

use chrono::Utc;
use sha2::{Digest, Sha256};
use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;

use crate::{
    audit,
    error::{AppError, AppResult},
    identity::ActorContext,
};

const MAX_DOCUMENT_BYTES: usize = 10 * 1024 * 1024;

#[derive(Clone, Debug, FromRow)]
pub struct ReleasedDocument {
    pub id: String,
    pub title: String,
    pub stored_name: String,
    pub original_name: String,
    pub media_type: String,
    pub byte_length: i64,
    pub sha256: String,
    pub released_at: String,
}

pub struct DocumentUpload<'a> {
    pub title: &'a str,
    pub original_name: &'a str,
    pub media_type: &'a str,
    pub bytes: &'a [u8],
}

pub async fn release_document(
    pool: &SqlitePool,
    data_dir: &Path,
    actor: &ActorContext,
    request_id: &str,
    upload: DocumentUpload<'_>,
) -> AppResult<()> {
    let DocumentUpload {
        title,
        original_name,
        media_type,
        bytes,
    } = upload;
    if !(2..=180).contains(&title.trim().chars().count())
        || bytes.is_empty()
        || bytes.len() > MAX_DOCUMENT_BYTES
    {
        return Err(AppError::Validation(
            "Invalid document title or file size".into(),
        ));
    }
    validate_document(media_type, bytes)?;
    let extension = match media_type {
        "application/pdf" => "pdf",
        "text/csv" => "csv",
        "text/plain" => "txt",
        "application/json" => "json",
        _ => return Err(AppError::Validation("Unsupported document type".into())),
    };
    let id = Uuid::new_v4().to_string();
    let stored_name = format!("{id}.{extension}");
    let path = data_dir.join("files").join(&stored_name);
    tokio::fs::write(&path, bytes)
        .await
        .map_err(|_| AppError::Internal)?;
    let sha256 = hex::encode(Sha256::digest(bytes));
    let inserted = sqlx::query("INSERT INTO released_documents(id,request_id,title,stored_name,original_name,media_type,byte_length,sha256,released_by,released_at) VALUES (?,?,?,?,?,?,?,?,?,?)")
        .bind(&id).bind(request_id).bind(title.trim()).bind(&stored_name).bind(safe_name(original_name)).bind(media_type).bind(bytes.len() as i64).bind(&sha256).bind(&actor.subject_id).bind(Utc::now().to_rfc3339()).execute(pool).await;
    if let Err(error) = inserted {
        let _ = tokio::fs::remove_file(&path).await;
        return Err(error.into());
    }
    audit::record(
        pool,
        Some(actor),
        audit::Entry {
            operation: "document.released",
            entity_type: "foi_request",
            entity_id: request_id,
            revision: None,
            outcome: "success",
            detail: serde_json::json!({"document_id":id,"sha256":sha256,"bytes":bytes.len()}),
        },
    )
    .await
}

fn validate_document(media_type: &str, bytes: &[u8]) -> AppResult<()> {
    let valid = match media_type {
        "application/pdf" => bytes.starts_with(b"%PDF-"),
        "application/json" => serde_json::from_slice::<serde_json::Value>(bytes).is_ok(),
        "text/csv" | "text/plain" => std::str::from_utf8(bytes).is_ok() && !bytes.contains(&0),
        _ => false,
    };
    if valid {
        Ok(())
    } else {
        Err(AppError::Validation(
            "Hindi tugma o hindi pinapayagan ang document format".into(),
        ))
    }
}

fn safe_name(value: &str) -> String {
    value
        .chars()
        .filter(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_' | ' ')
        })
        .take(120)
        .collect()
}

pub async fn list_documents(
    pool: &SqlitePool,
    request_id: &str,
) -> AppResult<Vec<ReleasedDocument>> {
    Ok(sqlx::query_as::<_, ReleasedDocument>("SELECT id,title,stored_name,original_name,media_type,byte_length,sha256,released_at FROM released_documents WHERE request_id=? ORDER BY released_at")
        .bind(request_id).fetch_all(pool).await?)
}

pub async fn load_document(pool: &SqlitePool, id: &str) -> AppResult<ReleasedDocument> {
    sqlx::query_as::<_, ReleasedDocument>("SELECT id,title,stored_name,original_name,media_type,byte_length,sha256,released_at FROM released_documents WHERE id=?")
        .bind(id).fetch_optional(pool).await?.ok_or(AppError::NotFound)
}

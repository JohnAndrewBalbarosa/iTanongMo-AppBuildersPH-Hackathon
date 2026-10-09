use chrono::{Datelike, Duration, NaiveDate, Weekday};
use sqlx::SqlitePool;

use crate::error::AppResult;

pub async fn initial_due_date(pool: &SqlitePool, submitted: NaiveDate) -> AppResult<NaiveDate> {
    add_working_days(pool, submitted, 15).await
}

pub async fn add_working_days(
    pool: &SqlitePool,
    start: NaiveDate,
    working_days: u32,
) -> AppResult<NaiveDate> {
    let holidays: Vec<String> = sqlx::query_scalar("SELECT holiday_date FROM holidays")
        .fetch_all(pool)
        .await?;
    let mut date = start;
    let mut counted = 0;
    while counted < working_days {
        date += Duration::days(1);
        let weekday = date.weekday();
        let weekend = matches!(weekday, Weekday::Sat | Weekday::Sun);
        if !weekend && !holidays.iter().any(|holiday| holiday == &date.to_string()) {
            counted += 1;
        }
    }
    Ok(date)
}

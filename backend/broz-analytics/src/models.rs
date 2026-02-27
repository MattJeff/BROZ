use chrono::{DateTime, NaiveDate, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::schema::{analytics_events, daily_stats};

// --- Analytics Events ---

#[derive(Debug, Queryable, Identifiable, Serialize)]
#[diesel(table_name = analytics_events)]
pub struct AnalyticsEvent {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub event_type: String,
    pub properties: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = analytics_events)]
pub struct NewAnalyticsEvent {
    pub user_id: Option<Uuid>,
    pub event_type: String,
    pub properties: Option<serde_json::Value>,
}

// --- Daily Stats ---

#[derive(Debug, Queryable, Serialize, Deserialize)]
#[diesel(table_name = daily_stats)]
pub struct DailyStat {
    pub date: NaiveDate,
    pub metric: String,
    pub value: i64,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = daily_stats)]
pub struct NewDailyStat {
    pub date: NaiveDate,
    pub metric: String,
    pub value: i64,
}

/// Upsert a daily stat using ON CONFLICT (date, metric) DO UPDATE SET value = EXCLUDED.value
pub fn upsert_daily_stat(
    conn: &mut diesel::pg::PgConnection,
    stat: &NewDailyStat,
) -> Result<(), diesel::result::Error> {
    diesel::sql_query(
        "INSERT INTO daily_stats (date, metric, value) \
         VALUES ($1, $2, $3) \
         ON CONFLICT (date, metric) DO UPDATE SET value = EXCLUDED.value"
    )
    .bind::<diesel::sql_types::Date, _>(stat.date)
    .bind::<diesel::sql_types::VarChar, _>(&stat.metric)
    .bind::<diesel::sql_types::BigInt, _>(stat.value)
    .execute(conn)?;
    Ok(())
}

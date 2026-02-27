use std::sync::Arc;
use chrono::{NaiveDate, Utc};
use diesel::prelude::*;
use diesel::sql_types::{BigInt, Text, Date as DieselDate};

use crate::AppState;
use crate::models::{NewDailyStat, upsert_daily_stat};

/// Aggregate daily stats from analytics_events and upsert into daily_stats.
///
/// Computes the following metrics:
/// - dau: distinct user_ids from today's events
/// - wau: distinct user_ids from the last 7 days
/// - mau: distinct user_ids from the last 30 days
/// - registrations_today: count of "broz.auth.user.registered" events today
/// - matches_today: count of "broz.matching.session.started" events today
pub fn aggregate_daily_stats(pool: &crate::DbPool) -> anyhow::Result<()> {
    let mut conn = pool.get()?;
    let today = Utc::now().date_naive();

    // DAU - distinct users today
    let dau = count_distinct_users_since(&mut conn, today, 0)?;
    upsert_daily_stat(&mut conn, &NewDailyStat {
        date: today,
        metric: "dau".to_string(),
        value: dau,
    })?;

    // WAU - distinct users last 7 days
    let wau = count_distinct_users_since(&mut conn, today, 7)?;
    upsert_daily_stat(&mut conn, &NewDailyStat {
        date: today,
        metric: "wau".to_string(),
        value: wau,
    })?;

    // MAU - distinct users last 30 days
    let mau = count_distinct_users_since(&mut conn, today, 30)?;
    upsert_daily_stat(&mut conn, &NewDailyStat {
        date: today,
        metric: "mau".to_string(),
        value: mau,
    })?;

    // Registrations today
    let registrations = count_events_today(&mut conn, today, "broz.auth.user.registered")?;
    upsert_daily_stat(&mut conn, &NewDailyStat {
        date: today,
        metric: "registrations_today".to_string(),
        value: registrations,
    })?;

    // Matches today
    let matches = count_events_today(&mut conn, today, "broz.matching.session.started")?;
    upsert_daily_stat(&mut conn, &NewDailyStat {
        date: today,
        metric: "matches_today".to_string(),
        value: matches,
    })?;

    tracing::info!(
        dau = dau,
        wau = wau,
        mau = mau,
        registrations = registrations,
        matches = matches,
        "daily stats aggregated"
    );

    Ok(())
}

/// Count distinct user_ids from analytics_events within `days_back` days from `today`.
/// If days_back is 0, counts only today's events.
fn count_distinct_users_since(
    conn: &mut diesel::pg::PgConnection,
    today: NaiveDate,
    days_back: i64,
) -> anyhow::Result<i64> {
    let start_date = if days_back == 0 {
        today
    } else {
        today - chrono::Duration::days(days_back)
    };

    #[derive(QueryableByName)]
    struct CountResult {
        #[diesel(sql_type = BigInt)]
        cnt: i64,
    }

    let result = diesel::sql_query(
        "SELECT COUNT(DISTINCT user_id) AS cnt \
         FROM analytics_events \
         WHERE user_id IS NOT NULL \
         AND created_at >= $1::date \
         AND created_at < ($2::date + INTERVAL '1 day')"
    )
    .bind::<DieselDate, _>(start_date)
    .bind::<DieselDate, _>(today)
    .get_result::<CountResult>(conn)?;

    Ok(result.cnt)
}

/// Count events of a specific type that occurred today.
fn count_events_today(
    conn: &mut diesel::pg::PgConnection,
    today: NaiveDate,
    event_type: &str,
) -> anyhow::Result<i64> {
    #[derive(QueryableByName)]
    struct CountResult {
        #[diesel(sql_type = BigInt)]
        cnt: i64,
    }

    let result = diesel::sql_query(
        "SELECT COUNT(*) AS cnt \
         FROM analytics_events \
         WHERE event_type = $1 \
         AND created_at >= $2::date \
         AND created_at < ($2::date + INTERVAL '1 day')"
    )
    .bind::<Text, _>(event_type)
    .bind::<DieselDate, _>(today)
    .get_result::<CountResult>(conn)?;

    Ok(result.cnt)
}

/// Spawn a background task that runs aggregate_daily_stats every hour.
pub fn spawn_aggregation_task(state: Arc<AppState>) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(3600));

        loop {
            interval.tick().await;

            tracing::info!("running hourly stats aggregation");
            match aggregate_daily_stats(&state.db) {
                Ok(()) => {
                    tracing::info!("hourly stats aggregation completed");
                }
                Err(e) => {
                    tracing::error!(error = %e, "hourly stats aggregation failed");
                }
            }
        }
    });
}

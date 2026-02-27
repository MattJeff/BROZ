use diesel::prelude::*;
use uuid::Uuid;

use broz_shared::clients::db::DbPool;
use broz_shared::errors::AppResult;

use crate::models::{NewNotification, Notification};
use crate::schema::notifications;

/// Create a new notification and insert it into the database.
pub fn create_notification(
    pool: &DbPool,
    user_id: Uuid,
    notification_type: &str,
    title: &str,
    body: &str,
    data: Option<serde_json::Value>,
) -> AppResult<Notification> {
    let mut conn = pool.get().map_err(|e| {
        tracing::error!(error = %e, "failed to get db connection");
        broz_shared::errors::AppError::internal("database connection error")
    })?;

    let new_notification = NewNotification {
        user_id,
        notification_type: notification_type.to_string(),
        title: title.to_string(),
        body: body.to_string(),
        data,
    };

    let notification = diesel::insert_into(notifications::table)
        .values(&new_notification)
        .get_result::<Notification>(&mut conn)?;

    tracing::debug!(
        notification_id = %notification.id,
        user_id = %user_id,
        notification_type = %notification_type,
        "notification created"
    );

    Ok(notification)
}

/// List notifications for a user with pagination.
pub fn list_notifications(
    pool: &DbPool,
    user_id: Uuid,
    limit: i64,
    offset: i64,
) -> AppResult<(Vec<Notification>, i64)> {
    let mut conn = pool.get().map_err(|e| {
        tracing::error!(error = %e, "failed to get db connection");
        broz_shared::errors::AppError::internal("database connection error")
    })?;

    let total: i64 = notifications::table
        .filter(notifications::user_id.eq(user_id))
        .count()
        .get_result(&mut conn)?;

    let items = notifications::table
        .filter(notifications::user_id.eq(user_id))
        .order(notifications::created_at.desc())
        .limit(limit)
        .offset(offset)
        .load::<Notification>(&mut conn)?;

    Ok((items, total))
}

/// Count unread notifications for a user.
pub fn count_unread(pool: &DbPool, user_id: Uuid) -> AppResult<i64> {
    let mut conn = pool.get().map_err(|e| {
        tracing::error!(error = %e, "failed to get db connection");
        broz_shared::errors::AppError::internal("database connection error")
    })?;

    let count: i64 = notifications::table
        .filter(notifications::user_id.eq(user_id))
        .filter(notifications::is_read.eq(false))
        .count()
        .get_result(&mut conn)?;

    Ok(count)
}

/// Mark all unread notifications as read for a user.
pub fn mark_all_read(pool: &DbPool, user_id: Uuid) -> AppResult<usize> {
    let mut conn = pool.get().map_err(|e| {
        tracing::error!(error = %e, "failed to get db connection");
        broz_shared::errors::AppError::internal("database connection error")
    })?;

    let updated = diesel::update(
        notifications::table
            .filter(notifications::user_id.eq(user_id))
            .filter(notifications::is_read.eq(false)),
    )
    .set(notifications::is_read.eq(true))
    .execute(&mut conn)?;

    Ok(updated)
}

/// Mark a single notification as read (only if it belongs to the user).
pub fn mark_read(pool: &DbPool, notification_id: Uuid, user_id: Uuid) -> AppResult<Notification> {
    let mut conn = pool.get().map_err(|e| {
        tracing::error!(error = %e, "failed to get db connection");
        broz_shared::errors::AppError::internal("database connection error")
    })?;

    let notification = diesel::update(
        notifications::table
            .filter(notifications::id.eq(notification_id))
            .filter(notifications::user_id.eq(user_id)),
    )
    .set(notifications::is_read.eq(true))
    .get_result::<Notification>(&mut conn)
    .map_err(|e| match e {
        diesel::result::Error::NotFound => broz_shared::errors::AppError::new(
            broz_shared::errors::ErrorCode::NotificationNotFound,
            "notification not found",
        ),
        other => broz_shared::errors::AppError::Database(other),
    })?;

    Ok(notification)
}

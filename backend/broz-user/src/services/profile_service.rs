use diesel::prelude::*;
use uuid::Uuid;

use broz_shared::errors::{AppError, AppResult};

use crate::models::{NewProfile, Profile};
use crate::schema::profiles;
use crate::DbPool;

/// Creates a default profile for a newly registered user.
/// Called from the RabbitMQ subscriber when a `user.registered` event is received.
pub fn create_default_profile(pool: &DbPool, credential_id: Uuid, _email: &str) -> AppResult<Profile> {
    let mut conn = pool.get().map_err(|e| AppError::internal(e.to_string()))?;

    let new_profile = NewProfile {
        credential_id,
    };

    let profile = diesel::insert_into(profiles::table)
        .values(&new_profile)
        .get_result::<Profile>(&mut conn)?;

    tracing::info!(
        profile_id = %profile.id,
        credential_id = %credential_id,
        "default profile created"
    );

    Ok(profile)
}

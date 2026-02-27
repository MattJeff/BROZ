// @generated automatically by Diesel CLI.

diesel::table! {
    match_sessions (id) {
        id -> Uuid,
        user_a_id -> Uuid,
        user_b_id -> Uuid,
        started_at -> Timestamptz,
        ended_at -> Nullable<Timestamptz>,
        #[max_length = 50]
        end_reason -> Nullable<Varchar>,
        duration_secs -> Nullable<Int4>,
    }
}

diesel::table! {
    livecam_requests (id) {
        id -> Uuid,
        requester_id -> Uuid,
        target_id -> Uuid,
        #[max_length = 20]
        status -> Varchar,
        #[max_length = 100]
        room_id -> Nullable<Varchar>,
        expires_at -> Timestamptz,
        responded_at -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    match_sessions,
    livecam_requests,
);

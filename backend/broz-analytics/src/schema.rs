// @generated automatically by Diesel CLI.

diesel::table! {
    analytics_events (id) {
        id -> Uuid,
        user_id -> Nullable<Uuid>,
        #[max_length = 255]
        event_type -> Varchar,
        properties -> Nullable<Jsonb>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    daily_stats (date, metric) {
        date -> Date,
        #[max_length = 100]
        metric -> Varchar,
        value -> Int8,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    analytics_events,
    daily_stats,
);

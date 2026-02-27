// @generated automatically by Diesel CLI.

diesel::table! {
    notifications (id) {
        id -> Uuid,
        user_id -> Uuid,
        #[max_length = 50]
        notification_type -> Varchar,
        #[max_length = 255]
        title -> Varchar,
        body -> Text,
        data -> Nullable<Jsonb>,
        is_read -> Bool,
        created_at -> Timestamptz,
    }
}

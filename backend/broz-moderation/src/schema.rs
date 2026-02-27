// @generated automatically by Diesel CLI.

diesel::table! {
    reports (id) {
        id -> Uuid,
        reporter_id -> Uuid,
        reported_id -> Uuid,
        #[max_length = 50]
        report_type -> Varchar,
        reason -> Text,
        context -> Nullable<Text>,
        match_session_id -> Nullable<Uuid>,
        message_id -> Nullable<Uuid>,
        #[max_length = 20]
        status -> Varchar,
        reviewed_by -> Nullable<Uuid>,
        reviewed_at -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    sanctions (id) {
        id -> Uuid,
        user_id -> Uuid,
        report_id -> Nullable<Uuid>,
        #[max_length = 20]
        sanction_type -> Varchar,
        reason -> Text,
        issued_by -> Uuid,
        expires_at -> Nullable<Timestamptz>,
        is_active -> Bool,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    admin_actions (id) {
        id -> Uuid,
        admin_id -> Uuid,
        #[max_length = 100]
        action -> Varchar,
        target_user_id -> Nullable<Uuid>,
        details -> Nullable<Jsonb>,
        created_at -> Timestamptz,
    }
}

diesel::joinable!(sanctions -> reports (report_id));

diesel::allow_tables_to_appear_in_same_query!(
    reports,
    sanctions,
    admin_actions,
);

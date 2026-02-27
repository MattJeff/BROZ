// @generated automatically by Diesel CLI.

diesel::table! {
    credentials (id) {
        id -> Uuid,
        #[max_length = 255]
        email -> Varchar,
        #[max_length = 255]
        password_hash -> Varchar,
        email_verified -> Bool,
        is_banned -> Bool,
        ban_until -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        #[max_length = 20]
        role -> Varchar,
    }
}

diesel::table! {
    oauth_accounts (id) {
        id -> Uuid,
        credential_id -> Uuid,
        #[max_length = 50]
        provider -> Varchar,
        #[max_length = 255]
        provider_uid -> Varchar,
        access_token_enc -> Nullable<Text>,
        refresh_token_enc -> Nullable<Text>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    email_verifications (id) {
        id -> Uuid,
        credential_id -> Uuid,
        #[max_length = 6]
        code -> Varchar,
        expires_at -> Timestamptz,
        used_at -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    password_resets (id) {
        id -> Uuid,
        credential_id -> Uuid,
        #[max_length = 6]
        code -> Varchar,
        expires_at -> Timestamptz,
        used_at -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    refresh_tokens (id) {
        id -> Uuid,
        credential_id -> Uuid,
        #[max_length = 255]
        token_hash -> Varchar,
        #[max_length = 255]
        device_fingerprint -> Nullable<Varchar>,
        expires_at -> Timestamptz,
        revoked_at -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
    }
}

diesel::joinable!(oauth_accounts -> credentials (credential_id));
diesel::joinable!(email_verifications -> credentials (credential_id));
diesel::joinable!(password_resets -> credentials (credential_id));
diesel::joinable!(refresh_tokens -> credentials (credential_id));

diesel::allow_tables_to_appear_in_same_query!(
    credentials,
    oauth_accounts,
    email_verifications,
    password_resets,
    refresh_tokens,
);

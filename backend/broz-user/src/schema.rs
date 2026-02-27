// @generated automatically by Diesel CLI.

diesel::table! {
    profiles (id) {
        id -> Uuid,
        credential_id -> Uuid,
        #[max_length = 20]
        display_name -> Nullable<Varchar>,
        bio -> Nullable<Text>,
        birth_date -> Nullable<Date>,
        profile_photo_url -> Nullable<Text>,
        kinks -> Jsonb,
        onboarding_complete -> Bool,
        #[max_length = 20]
        moderation_status -> Varchar,
        total_likes -> Int4,
        #[max_length = 3]
        country -> Nullable<Varchar>,
        is_online -> Bool,
        last_seen_at -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    follows (id) {
        id -> Uuid,
        follower_id -> Uuid,
        following_id -> Uuid,
        #[max_length = 20]
        status -> Varchar,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    likes (id) {
        id -> Uuid,
        liker_id -> Uuid,
        liked_id -> Uuid,
        match_session_id -> Nullable<Uuid>,
        created_at -> Timestamptz,
    }
}

diesel::joinable!(follows -> profiles (follower_id));
diesel::joinable!(likes -> profiles (liker_id));

diesel::allow_tables_to_appear_in_same_query!(
    profiles,
    follows,
    likes,
);

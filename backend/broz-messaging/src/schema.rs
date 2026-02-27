// @generated automatically by Diesel CLI.

diesel::table! {
    conversations (id) {
        id -> Uuid,
        is_group -> Bool,
        #[max_length = 100]
        group_name -> Nullable<Varchar>,
        group_photo_url -> Nullable<Text>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    conversation_members (id) {
        id -> Uuid,
        conversation_id -> Uuid,
        user_id -> Uuid,
        joined_at -> Timestamptz,
        last_read_at -> Timestamptz,
    }
}

diesel::table! {
    messages (id) {
        id -> Uuid,
        conversation_id -> Uuid,
        sender_id -> Uuid,
        content -> Nullable<Text>,
        media_url -> Nullable<Text>,
        #[max_length = 20]
        media_type -> Nullable<Varchar>,
        is_deleted -> Bool,
        is_private -> Bool,
        created_at -> Timestamptz,
    }
}

diesel::joinable!(conversation_members -> conversations (conversation_id));
diesel::joinable!(messages -> conversations (conversation_id));

diesel::allow_tables_to_appear_in_same_query!(
    conversations,
    conversation_members,
    messages,
);

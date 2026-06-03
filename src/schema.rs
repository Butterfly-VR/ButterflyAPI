// @generated automatically by Diesel CLI.

diesel::table! {
    chat_session_members (session, user) {
        session -> Uuid,
        user -> Uuid,
        last_seen_message -> Nullable<Uuid>,
        joined_at -> Timestamp,
    }
}

diesel::table! {
    chat_session_messages (id) {
        id -> Uuid,
        session -> Uuid,
        user -> Nullable<Uuid>,
        #[max_length = 4096]
        content -> Varchar,
        sent_at -> Timestamp,
        modified_at -> Nullable<Timestamp>,
        delete_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    instances (id) {
        id -> Uuid,
        server_token -> Bytea,
        world -> Uuid,
        #[max_length = 32]
        name -> Varchar,
        max_players -> Int2,
        publicity -> Int2,
        anyone_can_invite -> Bool,
        is_gameserver -> Bool,
        ip -> Inet,
        port -> Int4,
        created_at -> Timestamp,
    }
}

diesel::table! {
    ip_addresses (user, ip) {
        user -> Uuid,
        ip -> Inet,
        first_seen -> Timestamp,
    }
}

diesel::table! {
    licenses (id) {
        id -> Uuid,
        text -> Text,
    }
}

diesel::table! {
    moderations (id) {
        id -> Uuid,
        target -> Uuid,
        moderator -> Nullable<Uuid>,
        #[sql_name = "type"]
        type_ -> Int2,
        created_at -> Timestamp,
        expires -> Nullable<Timestamp>,
        details -> Nullable<Text>,
    }
}

diesel::table! {
    notifications (id) {
        id -> Uuid,
        target -> Nullable<Uuid>,
        #[sql_name = "type"]
        type_ -> Int2,
        #[max_length = 128]
        header -> Nullable<Varchar>,
        body -> Nullable<Text>,
        additional_data -> Nullable<Jsonb>,
        created_at -> Timestamp,
        expires -> Nullable<Timestamp>,
    }
}

diesel::table! {
    objects (id) {
        id -> Uuid,
        #[max_length = 32]
        name -> Varchar,
        #[max_length = 4096]
        description -> Varchar,
        flags -> Array<Nullable<Bool>>,
        updated_at -> Timestamp,
        created_at -> Timestamp,
        verified -> Bool,
        object_size -> Int8,
        image_size -> Int8,
        creator -> Uuid,
        object_type -> Int2,
        likes -> Int4,
        dislikes -> Int4,
        publicity -> Int2,
        license -> Uuid,
        encryption_key -> Bytea,
        encryption_iv -> Bytea,
        delete_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    tags (tag, object) {
        #[max_length = 32]
        tag -> Varchar,
        object -> Uuid,
    }
}

diesel::table! {
    tokens (token) {
        token -> Bytea,
        user -> Uuid,
        renewable -> Bool,
        expires -> Timestamp,
        last_used -> Timestamp,
    }
}

diesel::table! {
    unverified_users (id) {
        id -> Uuid,
        #[max_length = 32]
        username -> Varchar,
        #[max_length = 128]
        email -> Varchar,
        password -> Bytea,
        salt -> Bytea,
        token -> Bytea,
        expires -> Timestamp,
        created_at -> Timestamp,
    }
}

diesel::table! {
    user_reports (id) {
        id -> Uuid,
        reporter -> Uuid,
        target -> Uuid,
        target_type -> Int2,
        report_type -> Int2,
        #[max_length = 4096]
        details -> Varchar,
        additional_data -> Nullable<Jsonb>,
        created_at -> Timestamp,
        resolved -> Bool,
    }
}

diesel::table! {
    users (id) {
        id -> Uuid,
        #[max_length = 32]
        username -> Varchar,
        #[max_length = 128]
        email -> Varchar,
        password -> Bytea,
        salt -> Bytea,
        permissions_level -> Int2,
        trust -> Int4,
        homeworld -> Nullable<Uuid>,
        avatar -> Nullable<Uuid>,
        instance -> Nullable<Uuid>,
        identifier -> Nullable<Bytea>,
        created_at -> Timestamp,
        delete_at -> Nullable<Timestamp>,
        can_login -> Bool,
        upload_quota_used -> Int8,
        download_quota_used -> Int8,
    }
}

diesel::joinable!(chat_session_members -> users (user));
diesel::joinable!(chat_session_messages -> users (user));
diesel::joinable!(instances -> objects (world));
diesel::joinable!(ip_addresses -> users (user));
diesel::joinable!(notifications -> users (target));
diesel::joinable!(objects -> licenses (license));
diesel::joinable!(tags -> objects (object));
diesel::joinable!(tokens -> users (user));
diesel::joinable!(user_reports -> users (reporter));
diesel::joinable!(users -> instances (instance));

diesel::allow_tables_to_appear_in_same_query!(
    chat_session_members,
    chat_session_messages,
    instances,
    ip_addresses,
    licenses,
    moderations,
    notifications,
    objects,
    tags,
    tokens,
    unverified_users,
    user_reports,
    users,
);

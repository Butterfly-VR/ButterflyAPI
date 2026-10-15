// @generated automatically by Diesel CLI.

diesel::table! {
    chat_session_members (session, user) {
        session -> Uuid,
        user -> Uuid,
        last_seen_message -> Nullable<Uuid>,
        joined_at -> Timestamptz,
    }
}

diesel::table! {
    chat_session_messages (id) {
        id -> Uuid,
        session -> Uuid,
        user -> Nullable<Uuid>,
        #[max_length = 4096]
        content -> Varchar,
        sent_at -> Timestamptz,
        modified_at -> Nullable<Timestamptz>,
        delete_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    friends (user_id, friend_id) {
        user_id -> Uuid,
        friend_id -> Uuid,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    group_members (group_id, user_id) {
        group_id -> Uuid,
        user_id -> Uuid,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    groups (id) {
        id -> Uuid,
        #[max_length = 32]
        name -> Varchar,
        #[max_length = 4096]
        description -> Varchar,
        created_at -> Timestamptz,
        creator -> Uuid,
        publicity -> Int2,
        delete_at -> Nullable<Timestamptz>,
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
        created_at -> Timestamptz,
    }
}

diesel::table! {
    ip_addresses (user, ip) {
        user -> Uuid,
        ip -> Inet,
        first_seen -> Timestamptz,
    }
}

diesel::table! {
    ip_infos (ip) {
        ip -> Inet,
        accounts_created -> Int2,
        account_creation_count_reset -> Timestamptz,
        login_attempts -> Int2,
        login_attempts_reset -> Timestamptz,
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
        created_at -> Timestamptz,
        expires -> Nullable<Timestamptz>,
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
        created_at -> Timestamptz,
        expires -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    object_download_tokens (token) {
        token -> Uuid,
        object_id -> Uuid,
        current_step -> Int2,
        expiry -> Timestamptz,
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
        updated_at -> Timestamptz,
        created_at -> Timestamptz,
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
        delete_at -> Nullable<Timestamptz>,
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
        expires -> Timestamptz,
        last_used -> Timestamptz,
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
        expires -> Timestamptz,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    user_ips (user_id, ip) {
        user_id -> Uuid,
        ip -> Inet,
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
        created_at -> Timestamptz,
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
        created_at -> Timestamptz,
        delete_at -> Nullable<Timestamptz>,
        can_login -> Bool,
        upload_quota_used -> Int8,
        download_quota_used -> Int8,
        #[max_length = 4096]
        bio -> Nullable<Varchar>,
        status -> Int2,
        #[max_length = 32]
        status_message -> Nullable<Varchar>,
        #[max_length = 32]
        pronouns -> Nullable<Varchar>,
        #[max_length = 6]
        profile_main_colour -> Nullable<Varchar>,
        #[max_length = 6]
        profile_secondary_colour -> Nullable<Varchar>,
        profile_dark_mode -> Bool,
        profile_style -> Int2,
    }
}

diesel::joinable!(chat_session_members -> users (user));
diesel::joinable!(chat_session_messages -> users (user));
diesel::joinable!(instances -> objects (world));
diesel::joinable!(ip_addresses -> users (user));
diesel::joinable!(notifications -> users (target));
diesel::joinable!(object_download_tokens -> objects (object_id));
diesel::joinable!(objects -> licenses (license));
diesel::joinable!(tags -> objects (object));
diesel::joinable!(tokens -> users (user));
diesel::joinable!(user_ips -> ip_infos (ip));
diesel::joinable!(user_ips -> users (user_id));
diesel::joinable!(user_reports -> users (reporter));
diesel::joinable!(users -> instances (instance));

diesel::allow_tables_to_appear_in_same_query!(
    chat_session_members,
    chat_session_messages,
    friends,
    group_members,
    groups,
    instances,
    ip_addresses,
    ip_infos,
    licenses,
    moderations,
    notifications,
    object_download_tokens,
    objects,
    tags,
    tokens,
    unverified_users,
    user_ips,
    user_reports,
    users,
);

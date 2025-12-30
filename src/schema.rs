// @generated automatically by Diesel CLI.

diesel::table! {
    licenses (license) {
        license -> Int4,
        #[max_length = 100000]
        text -> Varchar,
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
        publicity -> Int2,
        license -> Int4,
        encryption_key -> Bytea,
        encryption_iv -> Bytea,
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
        expiry -> Timestamp,
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
        expiry -> Timestamp,
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
        permisions -> Array<Nullable<Bool>>,
        trust -> Int4,
        homeworld -> Nullable<Uuid>,
        avatar -> Nullable<Uuid>,
    }
}

diesel::joinable!(objects -> licenses (license));
diesel::joinable!(tags -> objects (object));
diesel::joinable!(tokens -> users (user));

diesel::allow_tables_to_appear_in_same_query!(
    licenses,
    objects,
    tags,
    tokens,
    unverified_users,
    users,
);

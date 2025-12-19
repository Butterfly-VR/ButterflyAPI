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
        object_size -> Int4,
        image_size -> Int4,
        creator -> Uuid,
        object_type -> Int2,
        publicity -> Int2,
        license -> Int4,
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
    tokens (user, token) {
        user -> Uuid,
        token -> Bytea,
        renewable -> Bool,
        expiry -> Nullable<Timestamp>,
    }
}

diesel::table! {
    users (id) {
        id -> Uuid,
        #[max_length = 32]
        username -> Varchar,
        email -> Text,
        password -> Bytea,
        salt -> Bytea,
        permisions -> Int2,
        trust -> Int4,
        verified_email -> Bool,
        homeworld -> Nullable<Uuid>,
        avatar -> Nullable<Uuid>,
    }
}

diesel::joinable!(objects -> licenses (license));
diesel::joinable!(tags -> objects (object));
diesel::joinable!(tokens -> users (user));

diesel::allow_tables_to_appear_in_same_query!(licenses, objects, tags, tokens, users,);

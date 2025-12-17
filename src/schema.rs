// @generated automatically by Diesel CLI.

diesel::table! {
    objects (id) {
        id -> Uuid,
        #[max_length = 20]
        name -> Varchar,
        #[max_length = 512]
        description -> Varchar,
        flags -> Array<Nullable<Bool>>,
        updated_at -> Timestamp,
        created_at -> Timestamp,
        verified -> Bool,
        object_size -> Int4,
        image_size -> Int4,
        creator -> Uuid,
        object_type -> Int2,
    }
}

diesel::table! {
    tags (tag, object) {
        #[max_length = 16]
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
        #[max_length = 20]
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

diesel::joinable!(tags -> objects (object));
diesel::joinable!(tokens -> users (user));

diesel::allow_tables_to_appear_in_same_query!(objects, tags, tokens, users,);

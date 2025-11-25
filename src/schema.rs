// @generated automatically by Diesel CLI.

pub mod sql_types {
    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "permision_level"))]
    pub struct PermisionLevel;

    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "varbit", schema = "pg_catalog"))]
    pub struct Varbit;
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::Varbit;

    objects (id) {
        id -> Uuid,
        #[max_length = 20]
        name -> Varchar,
        #[max_length = 512]
        description -> Varchar,
        flags -> Varbit,
        updated_at -> Timestamp,
        created_at -> Timestamp,
        verified -> Bool,
        object_size -> Int4,
        image_size -> Int4,
        object_id -> Uuid,
        image_id -> Uuid,
        creator -> Uuid,
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
        expiry -> Nullable<Timestamp>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::PermisionLevel;

    users (id) {
        id -> Uuid,
        #[max_length = 20]
        username -> Varchar,
        email -> Text,
        password -> Bytea,
        salt -> Bytea,
        permisions -> PermisionLevel,
        trust -> Int4,
        verified_email -> Bool,
        homeworld -> Nullable<Uuid>,
        avatar -> Nullable<Uuid>,
    }
}

diesel::joinable!(tags -> objects (object));
diesel::joinable!(tokens -> users (user));

diesel::allow_tables_to_appear_in_same_query!(objects, tags, tokens, users,);

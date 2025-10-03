// @generated automatically by Diesel CLI.

diesel::table! {
    temp_test_users (id) {
        id -> Int4,
        #[max_length = 32]
        username -> Varchar,
        password -> Bytea,
        salt -> Bytea,
        #[max_length = 255]
        email -> Varchar,
    }
}

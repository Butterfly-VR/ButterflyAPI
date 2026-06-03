use bb8::PooledConnection;
use diesel::{ExpressionMethods, QueryDsl, delete};
use diesel_async::{
    AsyncPgConnection, RunQueryDsl, pooled_connection::AsyncDieselConnectionManager,
};

use crate::schema::{chat_session_messages, notifications, tokens, unverified_users, users};

pub async fn run_expiry_cleanup(
    conn: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
) {
    delete(users::table.filter(users::delete_at.le(diesel::dsl::now)))
        .execute(conn)
        .await
        .unwrap();

    delete(
        chat_session_messages::table.filter(chat_session_messages::delete_at.le(diesel::dsl::now)),
    )
    .execute(conn)
    .await
    .unwrap();

    delete(notifications::table.filter(notifications::expires.le(diesel::dsl::now)))
        .execute(conn)
        .await
        .unwrap();

    delete(unverified_users::table.filter(unverified_users::expires.le(diesel::dsl::now)))
        .execute(conn)
        .await
        .unwrap();

    delete(tokens::table.filter(tokens::expires.le(diesel::dsl::now)))
        .execute(conn)
        .await
        .unwrap();

    delete(unverified_users::table.filter(unverified_users::expires.le(diesel::dsl::now)))
        .execute(conn)
        .await
        .unwrap();
}

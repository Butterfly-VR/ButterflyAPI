use bb8::PooledConnection;
use chrono::Utc;
use diesel::{ExpressionMethods, QueryDsl, delete};
use diesel_async::{
    AsyncPgConnection, RunQueryDsl, pooled_connection::AsyncDieselConnectionManager,
};

use crate::schema::{
    chat_session_messages, notifications, object_download_tokens, tokens, unverified_users, users,
};

pub async fn run_expiry_cleanup(
    conn: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
) {
    delete(users::table.filter(users::delete_at.le(Utc::now())))
        .execute(conn)
        .await
        .unwrap();

    delete(chat_session_messages::table.filter(chat_session_messages::delete_at.le(Utc::now())))
        .execute(conn)
        .await
        .unwrap();

    delete(notifications::table.filter(notifications::expires.le(Utc::now())))
        .execute(conn)
        .await
        .unwrap();

    delete(unverified_users::table.filter(unverified_users::expires.le(Utc::now())))
        .execute(conn)
        .await
        .unwrap();

    delete(tokens::table.filter(tokens::expires.le(Utc::now())))
        .execute(conn)
        .await
        .unwrap();

    delete(unverified_users::table.filter(unverified_users::expires.le(Utc::now())))
        .execute(conn)
        .await
        .unwrap();

    delete(object_download_tokens::table.filter(object_download_tokens::expiry.lt(Utc::now())))
        .execute(conn)
        .await
        .unwrap();
}

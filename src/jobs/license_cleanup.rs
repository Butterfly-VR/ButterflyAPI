use crate::schema::{licenses, objects};
use bb8::PooledConnection;
use diesel::{ExpressionMethods, QueryDsl, delete};
use diesel_async::{
    AsyncPgConnection, RunQueryDsl, pooled_connection::AsyncDieselConnectionManager,
};

pub async fn run_license_cleanup(
    conn: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
) {
    delete(licenses::table)
        .filter(licenses::id.ne_all(objects::table.select(objects::license)))
        .execute(conn)
        .await
        .unwrap();
}

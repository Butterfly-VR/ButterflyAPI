use crate::schema::{instances, users};
use bb8::PooledConnection;
use diesel::{ExpressionMethods, NullableExpressionMethods, QueryDsl, delete};
use diesel_async::{
    AsyncPgConnection, RunQueryDsl, pooled_connection::AsyncDieselConnectionManager,
};

pub async fn run_instance_cleanup(
    conn: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
) {
    delete(instances::table)
        .filter(
            instances::id.nullable().ne_all(
                users::table
                    .select(users::instance)
                    .filter(users::instance.is_not_null()),
            ),
        )
        .execute(conn)
        .await
        .unwrap();
}

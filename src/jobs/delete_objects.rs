use bb8::PooledConnection;
use diesel::{ExpressionMethods, delete};
use diesel_async::{
    AsyncPgConnection, RunQueryDsl, pooled_connection::AsyncDieselConnectionManager,
};
use uuid::Uuid;

use crate::{models::ObjectType, schema::objects};

pub async fn run_delete_objects(
    conn: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    s3_client: aws_sdk_s3::Client,
) {
    const OBJECT_IMAGE_SUFFIX: &str = "-images";

    let object_list: Vec<(i16, Uuid)> = delete(objects::table)
        .filter(objects::delete_at.le(diesel::dsl::now))
        .returning((objects::object_type, objects::id))
        .get_results::<(i16, Uuid)>(conn)
        .await
        .unwrap();

    for (object_type, id) in object_list.iter().copied() {
        let object_type: ObjectType = object_type.try_into().unwrap();

        let bucket: &str = object_type.into();
        let key = id;
        s3_client
            .delete_object()
            .bucket(bucket)
            .key(key)
            .send()
            .await
            .unwrap();

        let bucket = bucket.to_string() + OBJECT_IMAGE_SUFFIX;
        s3_client
            .delete_object()
            .bucket(bucket)
            .key(key)
            .send()
            .await
            .unwrap();
    }
}

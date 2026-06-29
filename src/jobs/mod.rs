use bb8::Pool;
use diesel_async::{AsyncPgConnection, pooled_connection::AsyncDieselConnectionManager};

mod delete_objects;
mod expiry_cleanup;
mod instance_cleanup;
mod license_cleanup;

pub async fn run_all_jobs(
    pool: Pool<AsyncDieselConnectionManager<AsyncPgConnection>>,
    s3_client: aws_sdk_s3::Client,
) {
    delete_objects::run_delete_objects(&mut pool.get().await.unwrap(), s3_client).await;
    expiry_cleanup::run_expiry_cleanup(&mut pool.get().await.unwrap()).await;
    instance_cleanup::run_instance_cleanup(&mut pool.get().await.unwrap()).await;
    license_cleanup::run_license_cleanup(&mut pool.get().await.unwrap()).await;
}

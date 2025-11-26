use crate::AppState;
use crate::HASHER_ALGORITHM;
use crate::HASHER_PARAMETERS;
use crate::HASHER_VERSION;
use argon2::Argon2;
use axum;
use axum::extract::State;
use std::sync::Arc;
use tokio::task::spawn_blocking;

pub async fn hash_password(
    state: State<Arc<AppState>>,
    pwd: [u8; 64],
    slt: [u8; 64],
) -> Result<Vec<u8>, ()> {
    spawn_blocking(move || {
        let mut block = None;
        for lock in state.hasher_memory.iter() {
            if let Ok(b) = lock.try_lock() {
                block = Some(b);
                break;
            }
        }

        if block.is_none() {
            return Err(());
        }
        let block = block.unwrap();
        let mut out = vec![0_u8; 64];
        Argon2::new(
            HASHER_ALGORITHM,
            HASHER_VERSION,
            HASHER_PARAMETERS.clone().unwrap(),
        )
        .hash_password_into_with_memory(&pwd, &slt, &mut out, **block);
        Ok(out)
    })
    .await
    .unwrap_or(Err(()))
}

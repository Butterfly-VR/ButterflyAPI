use crate::AppState;
use argon2::Argon2;
use std::sync::Arc;
use tokio::task::spawn_blocking;
use tracing::{info, warn};

// password hasher parameters
// changing these could stop all users from logging in
pub const HASHER_MEMORY: u32 = 64_000; // 64MB
const HASHER_ITERATIONS: u32 = 10;
const HASHER_OUTPUT_LEN: u32 = 64;

const HASHER_ALGORITHM: argon2::Algorithm = argon2::Algorithm::Argon2id;
const HASHER_VERSION: argon2::Version = argon2::Version::V0x13;
// todo: unwrap this here when const unwrap gets stabalized
static HASHER_PARAMETERS: Result<argon2::Params, argon2::Error> = argon2::Params::new(
    HASHER_MEMORY,
    HASHER_ITERATIONS,
    1,
    Some(HASHER_OUTPUT_LEN as usize),
);

pub async fn hash_password(
    state: Arc<AppState>,
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
            info!(
                "hasher failed to find block, this is expected if too many users sign up/in at once"
            );
            return Err(());
        }
        let mut block = block.unwrap();
        let mut out = vec![0_u8; 64];
        if Argon2::new(
            HASHER_ALGORITHM,
            HASHER_VERSION,
            HASHER_PARAMETERS.clone().unwrap(),
        )
        .hash_password_into_with_memory(&pwd, &slt, &mut out, block.as_mut_slice())
        .is_ok()
        {
            Ok(out)
        } else {
            warn!("unknown error while hashing");
            Err(())
        }
    })
    .await
    .unwrap_or(Err(()))
}

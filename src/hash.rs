use crate::AppState;
use argon2::Argon2;
use rand::seq::IndexedRandom;
use std::sync::Arc;
use tokio::task::spawn_blocking;
use tracing::warn;

// password hasher parameters
// changing these could stop all users from logging in
// these values might seem low but consider the main attack vector for a weak password hasher
// is a dictionary attack, the server operates on the hashed output from the client side hash
// therefore, an attacker would need to perform a brute force attack covering all possible hash outputs
// to crack a password.
pub const HASHER_MEMORY: u32 = 64_000;
const HASHER_ITERATIONS: u32 = 1;
const HASHER_OUTPUT_LEN: u32 = 64;

const HASHER_ALGORITHM: argon2::Algorithm = argon2::Algorithm::Argon2id;
const HASHER_VERSION: argon2::Version = argon2::Version::V0x13;

static HASHER_PARAMETERS: argon2::Params = match argon2::Params::new(
    HASHER_MEMORY,
    HASHER_ITERATIONS,
    1,
    Some(HASHER_OUTPUT_LEN as usize),
) {
    Ok(params) => params,
    Err(_) => {
        panic!("failed to create hasher parameters");
    }
};

// not sure what clippy wants here, the lock is dropped basically as soon as possible
#[allow(clippy::significant_drop_tightening)]
pub async fn hash_password(
    state: Arc<AppState>,
    pwd: [u8; 64],
    slt: [u8; 64],
) -> Result<Vec<u8>, ()> {
    spawn_blocking(move || {
        let mut block = loop {
            let mut block = None;
            for lock in &state.hasher_memory {
                if let Ok(b) = lock.try_lock() {
                    block = Some(b);
                    break;
                }
            }
            if block.is_some() {
                break block.unwrap();
            } else {
                // no lock available, choose a random one and block until it's available
                break state
                    .hasher_memory
                    .choose(&mut rand::rng())
                    .unwrap()
                    .lock()
                    .unwrap();
            }
        };
        let mut out = vec![0_u8; 64];
        if Argon2::new(HASHER_ALGORITHM, HASHER_VERSION, HASHER_PARAMETERS.clone())
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

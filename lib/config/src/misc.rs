use std::time::SystemTime;
use sha2::{Digest, Sha256};

pub fn get_epoch() -> u64 {
    let duration_since_epoch = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap();
    duration_since_epoch.as_secs()
}

pub fn sha256(serialized_data:Vec<u8>) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(&serialized_data);
    let result = hasher.finalize();
    result.into()
}
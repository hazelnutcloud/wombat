use diesel::{
    r2d2::{ConnectionManager, Pool},
    sqlite::SqliteConnection,
};
use rand::{distributions::Alphanumeric, Rng};
use sha2::{Digest, Sha256};

pub type DbPool = Pool<ConnectionManager<SqliteConnection>>;

pub fn generate_secret_key(length: usize) -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .collect()
}

pub fn hash_key(key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(key);
    format!("{:x}", hasher.finalize())
}

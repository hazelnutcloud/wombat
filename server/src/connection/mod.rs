use std::collections::HashMap;

use diesel::prelude::*;
use tokio::net::TcpStream;

use crate::utils::DbPool;

pub struct ConnectionHolder {
    connection_map: HashMap<String, TcpStream>,
    db_pool: DbPool,
}

impl ConnectionHolder {
    pub fn new(db_pool: DbPool) -> ConnectionHolder {
        ConnectionHolder {
            connection_map: HashMap::new(),
            db_pool,
        }
    }

    pub fn add_connection(&mut self, key_hash: String, conn: TcpStream) -> anyhow::Result<()> {
        use crate::schema::secret_keys::dsl::*;

        let mut db_conn = self.db_pool.get()?;

        let user_id_value: String = secret_keys
            .filter(secret_key_hash.eq(key_hash))
            .select(user_id)
            .first(&mut db_conn)?;

        self.connection_map.insert(user_id_value, conn);

        Ok(())
    }
}

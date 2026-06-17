use redis::aio::ConnectionManager;
use redis::{Client, RedisError};

/// Redis client wrapper
#[derive(Clone)]
pub struct RedisClient {
    manager: ConnectionManager,
}

impl RedisClient {
    /// Create a new Redis client with connection pooling
    pub async fn new(redis_url: &str) -> Result<Self, RedisError> {
        let client = Client::open(redis_url)?;
        let manager = ConnectionManager::new(client).await?;
        Ok(Self { manager })
    }

    /// Get a connection from the pool
    pub fn get_connection(&self) -> ConnectionManager {
        self.manager.clone()
    }
}

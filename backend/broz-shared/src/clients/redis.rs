use redis::aio::ConnectionManager;
use redis::AsyncCommands;

#[derive(Clone)]
pub struct RedisClient {
    conn: ConnectionManager,
}

impl RedisClient {
    pub async fn connect(url: &str) -> Result<Self, redis::RedisError> {
        let client = redis::Client::open(url)?;
        let conn = client.get_connection_manager().await?;
        tracing::info!(url = %url, "connected to Redis");
        Ok(Self { conn })
    }

    pub async fn get(&self, key: &str) -> Result<Option<String>, redis::RedisError> {
        let mut conn = self.conn.clone();
        conn.get(key).await
    }

    pub async fn set(&self, key: &str, value: &str, ttl_secs: u64) -> Result<(), redis::RedisError> {
        let mut conn = self.conn.clone();
        conn.set_ex(key, value, ttl_secs).await
    }

    pub async fn del(&self, key: &str) -> Result<(), redis::RedisError> {
        let mut conn = self.conn.clone();
        conn.del(key).await
    }

    pub async fn exists(&self, key: &str) -> Result<bool, redis::RedisError> {
        let mut conn = self.conn.clone();
        conn.exists(key).await
    }

    pub async fn incr(&self, key: &str) -> Result<i64, redis::RedisError> {
        let mut conn = self.conn.clone();
        conn.incr(key, 1i64).await
    }

    pub async fn expire(&self, key: &str, ttl_secs: i64) -> Result<(), redis::RedisError> {
        let mut conn = self.conn.clone();
        conn.expire(key, ttl_secs).await
    }

    pub async fn set_nx(&self, key: &str, value: &str, ttl_secs: u64) -> Result<bool, redis::RedisError> {
        let mut conn = self.conn.clone();
        let set: bool = redis::cmd("SET")
            .arg(key)
            .arg(value)
            .arg("NX")
            .arg("EX")
            .arg(ttl_secs)
            .query_async(&mut conn)
            .await
            .unwrap_or(false);
        Ok(set)
    }

    pub async fn zadd(&self, key: &str, member: &str, score: f64) -> Result<(), redis::RedisError> {
        let mut conn = self.conn.clone();
        conn.zadd(key, member, score).await
    }

    pub async fn zrem(&self, key: &str, member: &str) -> Result<(), redis::RedisError> {
        let mut conn = self.conn.clone();
        conn.zrem(key, member).await
    }

    pub async fn zrange(&self, key: &str, start: isize, stop: isize) -> Result<Vec<String>, redis::RedisError> {
        let mut conn = self.conn.clone();
        conn.zrange(key, start, stop).await
    }

    pub async fn zcard(&self, key: &str) -> Result<u64, redis::RedisError> {
        let mut conn = self.conn.clone();
        conn.zcard(key).await
    }

    pub async fn rate_limit_check(
        &self,
        key: &str,
        limit: u64,
        window_secs: u64,
    ) -> Result<bool, redis::RedisError> {
        let mut conn = self.conn.clone();
        let count: u64 = conn.incr(key, 1u64).await?;
        if count == 1 {
            conn.expire::<_, ()>(key, window_secs as i64).await?;
        }
        Ok(count <= limit)
    }

    pub async fn mget(&self, keys: &[String]) -> Result<Vec<Option<String>>, redis::RedisError> {
        if keys.is_empty() {
            return Ok(vec![]);
        }
        let mut conn = self.conn.clone();
        redis::cmd("MGET").arg(keys).query_async(&mut conn).await
    }

    pub async fn exists_multi(&self, keys: &[String]) -> Result<Vec<bool>, redis::RedisError> {
        if keys.is_empty() {
            return Ok(vec![]);
        }
        let mut conn = self.conn.clone();
        let mut pipe = redis::pipe();
        for key in keys {
            pipe.exists(key.as_str());
        }
        pipe.query_async(&mut conn).await
    }

    pub fn connection(&self) -> ConnectionManager {
        self.conn.clone()
    }
}

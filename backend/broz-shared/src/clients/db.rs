use diesel::pg::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool};

pub type DbPool = Pool<ConnectionManager<PgConnection>>;

pub fn create_pool(database_url: &str) -> DbPool {
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    let pool = Pool::builder()
        .max_size(10)
        .min_idle(Some(2))
        .test_on_check_out(true)
        .build(manager)
        .expect("failed to create database pool");

    tracing::info!("database connection pool created");
    pool
}

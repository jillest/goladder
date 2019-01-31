use r2d2_postgres::{TlsMode, PostgresConnectionManager};

pub type Pool = r2d2::Pool<r2d2_postgres::PostgresConnectionManager>;
pub type Connection = r2d2::PooledConnection<r2d2_postgres::PostgresConnectionManager>;

pub fn create_pool() -> Pool {
    let manager = PostgresConnectionManager::new("postgresql://jilles@%2Ftmp/goladder", TlsMode::None).unwrap();
    Pool::builder()
        .max_size(2)
        .build(manager)
        .unwrap()
}

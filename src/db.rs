use std::ffi::OsStr;

use r2d2_sqlite::SqliteConnectionManager;

pub type Pool = r2d2::Pool<r2d2_sqlite::SqliteConnectionManager>;
pub type Connection = r2d2::PooledConnection<r2d2_sqlite::SqliteConnectionManager>;

pub fn create_pool(path: &OsStr) -> Pool {
    let manager = SqliteConnectionManager::file(path);
    Pool::builder().max_size(2).build(manager).unwrap()
}

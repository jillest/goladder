use std::ffi::OsStr;
use std::str::FromStr;

use r2d2_sqlite::SqliteConnectionManager;

use rusqlite::types::{FromSql, FromSqlError, ValueRef};

use crate::models::GameResult;

pub type Pool = r2d2::Pool<r2d2_sqlite::SqliteConnectionManager>;

pub fn create_pool(path: &OsStr) -> Result<Pool, r2d2::Error> {
    let manager = SqliteConnectionManager::file(path);
    Pool::builder().max_size(1).build(manager)
}

static SCHEMA: &str = include_str!("../database/schema.sql");

pub fn ensure_schema(conn: &rusqlite::Connection) -> rusqlite::Result<()> {
    match conn.prepare("SELECT white, black FROM games ORDER BY id") {
        Ok(_) => Ok(()),
        Err(_) => {
            eprintln!("note: initializing database");
            conn.execute_batch(SCHEMA)
        }
    }
}

impl FromSql for GameResult {
    fn column_result(val: ValueRef) -> Result<Self, FromSqlError> {
        GameResult::from_str(val.as_str()?).map_err(|e| FromSqlError::Other(Box::new(e)))
    }
}

#[cfg(test)]
mod tests {
    use crate::models::GameResult;
    use rusqlite::types::{FromSql, ValueRef};
    use super::*;

    #[test]
    fn game_result_from_sql_ok() {
        let val = ValueRef::Text("BlackWins");
        let gr: GameResult = FromSql::column_result(val).unwrap();
        assert_eq!(gr, GameResult::BlackWins);
    }

    #[test]
    fn game_result_from_sql_error_1() {
        let val = ValueRef::Text("wrong");
        let r: Result<GameResult, _> = FromSql::column_result(val);
        assert!(r.is_err());
    }

    #[test]
    fn game_result_from_sql_error_2() {
        let val = ValueRef::Integer(3);
        let r: Result<GameResult, _> = FromSql::column_result(val);
        assert!(r.is_err());
    }

    #[test]
    fn initialize_database() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        ensure_schema(&conn).unwrap();
    }
}

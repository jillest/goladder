use std::ffi::OsStr;
use std::str::FromStr;

use r2d2_sqlite::SqliteConnectionManager;

use rusqlite::types::{FromSql, FromSqlError, ToSql, ToSqlOutput, Value, ValueRef};

use crate::models::{GameResult, RoundExtra};

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

impl FromSql for RoundExtra {
    fn column_result(val: ValueRef) -> Result<Self, FromSqlError> {
        match val.as_str_or_null()? {
            None => Ok(Default::default()),
            Some(s) => {
                serde_json::from_str(s).map_err(|e| FromSqlError::Other(Box::new(e)))
            }
        }
    }
}

impl ToSql for RoundExtra {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        let s = serde_json::to_string(self)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
        Ok(ToSqlOutput::Owned(Value::Text(s)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::GameResult;
    use rusqlite::types::{FromSql, ValueRef};
    use serde_json::json;
    use std::collections::HashMap;
    use std::iter;

    #[test]
    fn game_result_from_sql_ok() {
        let val = ValueRef::Text(b"BlackWins");
        let gr: GameResult = FromSql::column_result(val).unwrap();
        assert_eq!(gr, GameResult::BlackWins);
    }

    #[test]
    fn game_result_from_sql_error_1() {
        let val = ValueRef::Text(b"wrong");
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
    fn round_extra_from_sql_null() {
        let val = ValueRef::Null;
        let re: RoundExtra = FromSql::column_result(val).unwrap();
        assert_eq!(re.unknown_fields.len(), 0);
    }

    #[test]
    fn round_extra_from_sql_empty() {
        let val = ValueRef::Text(b"{}");
        let re: RoundExtra = FromSql::column_result(val).unwrap();
        assert_eq!(re.unknown_fields.len(), 0);
    }

    #[test]
    fn round_extra_from_sql_unknown_field() {
        let val = ValueRef::Text(b"{\"unknown_field_for_test\": 8}");
        let re: RoundExtra = FromSql::column_result(val).unwrap();
        let expected_fields: HashMap<_, _> =
            iter::once(("unknown_field_for_test".to_owned(), json!(8))).collect();
        assert_eq!(re.unknown_fields, expected_fields);
    }

    #[test]
    fn round_extra_to_sql_1() {
        let re: RoundExtra = Default::default();
        let out = re.to_sql().unwrap();
        let s = match out {
            ToSqlOutput::Borrowed(ValueRef::Text(s)) => s,
            ToSqlOutput::Owned(Value::Text(ref s)) => s.as_bytes(),
            _ => panic!("incorrect type for RoundExtra SQL result in {:?}", out),
        };
        assert_eq!(&s[0..1], b"{");
    }

    #[test]
    fn initialize_database() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        ensure_schema(&conn).unwrap();
    }
}

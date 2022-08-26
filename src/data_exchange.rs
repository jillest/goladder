use actix_web::{http, HttpResponse};
use askama::Template;
use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::{CommonTemplate, Result};

#[derive(Deserialize, Serialize, Debug)]
struct Player {
    name: String,
    rating: f64,
    default_schedule: bool,
}

#[derive(Deserialize, Serialize, Debug)]
struct Data {
    #[serde(skip_deserializing)]
    program_version: &'static str,
    players: Vec<Player>,
}

pub(crate) fn export(conn: &rusqlite::Connection) -> Result<HttpResponse> {
    let s = export_internal(conn)?;
    Ok(HttpResponse::Ok()
        .content_type("application/json")
        .append_header((
            http::header::CONTENT_DISPOSITION,
            http::header::ContentDisposition {
                disposition: http::header::DispositionType::Attachment,
                parameters: vec![http::header::DispositionParam::Filename(
                    "goladder_export.json".to_owned(),
                )],
            },
        ))
        .body(s))
}

fn export_internal(conn: &rusqlite::Connection) -> Result<String> {
    let mut stmt = conn.prepare(concat!(
        "SELECT name, currentrating, defaultschedule FROM players ",
        "ORDER BY currentrating DESC, id"
    ))?;
    let players: Vec<Player> = stmt
        .query_map([], |row| {
            Ok(Player {
                name: row.get(0)?,
                rating: row.get(1)?,
                default_schedule: row.get(2)?,
            })
        })?
        .collect::<rusqlite::Result<_>>()?;
    let data = Data {
        program_version: env!("CARGO_PKG_VERSION"),
        players,
    };
    Ok(serde_json::to_string_pretty(&data)?)
}

#[derive(Template)]
#[template(path = "import.html")]
pub(crate) struct ImportTemplate {
    imported: usize,
    skipped: usize,
}
impl CommonTemplate for ImportTemplate {}

impl ImportTemplate {
    pub(crate) fn zero() -> Self {
        ImportTemplate {
            imported: 0,
            skipped: 0,
        }
    }
}

impl std::ops::Add for ImportTemplate {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        ImportTemplate {
            imported: self.imported + other.imported,
            skipped: self.skipped + other.skipped,
        }
    }
}

pub(crate) fn import(conn: &mut rusqlite::Connection, json: &str) -> Result<ImportTemplate> {
    import_internal(conn, json)
}

fn import_internal(conn: &mut rusqlite::Connection, json: &str) -> Result<ImportTemplate> {
    let data: Data = serde_json::from_str(json)?;
    let mut result = ImportTemplate::zero();
    let trans = conn.transaction()?;
    {
        let mut qstmt = trans.prepare("SELECT id FROM players WHERE name = ?1")?;
        let mut istmt = trans.prepare(concat!(
            "INSERT INTO players ",
            "(name, initialrating, currentrating, defaultschedule) ",
            "VALUES (?1, ?2, ?2, ?3)"
        ))?;
        for p in &data.players {
            if qstmt.exists(&[&p.name])? {
                result.skipped += 1;
            } else {
                istmt.execute(params![&p.name, &p.rating, &p.default_schedule])?;
                result.imported += 1;
            }
        }
    }
    trans.commit()?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::ensure_schema;

    #[test]
    fn export_0() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        ensure_schema(&conn).unwrap();
        let s = export_internal(&conn).unwrap();
        assert_eq!(s.chars().next(), Some('{'));
        let data: Data = serde_json::from_str(&s).unwrap();
        assert_eq!(data.players.len(), 0);
    }

    #[test]
    fn export_2() {
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        ensure_schema(&conn).unwrap();
        {
            let trans = conn.transaction().unwrap();
            trans
                .execute(
                    concat!(
                        "INSERT INTO players (id, name, initialrating, currentrating) VALUES ",
                        "(41, \"player1\", 1000.0, 1020.0), ",
                        "(42, \"player2\", 1000.0, 980.0);"
                    ),
                    [],
                )
                .unwrap();
            trans.commit().unwrap();
        }
        let s = export_internal(&conn).unwrap();
        assert_eq!(s.chars().next(), Some('{'));
        let data: Data = serde_json::from_str(&s).unwrap();
        assert_eq!(data.players.len(), 2);
        let p1 = &data.players[0];
        assert_eq!(p1.name, "player1");
        assert_eq!(p1.rating, 1020.0);
        assert_eq!(p1.default_schedule, false);
        let p2 = &data.players[1];
        assert_eq!(p2.name, "player2");
        assert_eq!(p2.rating, 980.0);
        assert_eq!(p2.default_schedule, false);
    }

    #[test]
    fn import_error_1() {
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        ensure_schema(&conn).unwrap();
        let r = import(&mut conn, "{");
        assert!(r.is_err());
    }

    #[test]
    fn import_error_2() {
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        ensure_schema(&conn).unwrap();
        let r = import(&mut conn, "{}");
        assert!(r.is_err());
    }

    #[test]
    fn export_import_0() {
        let s = {
            let conn = rusqlite::Connection::open_in_memory().unwrap();
            ensure_schema(&conn).unwrap();
            export_internal(&conn).unwrap()
        };
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        ensure_schema(&conn).unwrap();
        let r = import_internal(&mut conn, &s).unwrap();
        assert_eq!(r.imported, 0);
        assert_eq!(r.skipped, 0);
    }

    #[test]
    fn export_import_2() {
        let s = {
            let mut conn = rusqlite::Connection::open_in_memory().unwrap();
            ensure_schema(&conn).unwrap();
            {
                let trans = conn.transaction().unwrap();
                trans
                    .execute(
                        concat!(
                            "INSERT INTO players (id, name, initialrating, currentrating) VALUES ",
                            "(41, \"player1\", 1000.0, 1020.0), ",
                            "(42, \"player2\", 1000.0, 980.0);"
                        ),
                        [],
                    )
                    .unwrap();
                trans.commit().unwrap();
            }
            export_internal(&conn).unwrap()
        };
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        ensure_schema(&conn).unwrap();
        let r = import_internal(&mut conn, &s).unwrap();
        assert_eq!(r.imported, 2);
        assert_eq!(r.skipped, 0);
        let (ir, cr, ds): (f64, f64, bool) = conn
            .query_row(
                concat!(
                    "SELECT initialrating, currentrating, defaultschedule FROM players ",
                    "WHERE name = ?1"
                ),
                &["player1"],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(ir, 1020.0);
        assert_eq!(cr, 1020.0);
        assert_eq!(ds, false);
        let (ir, cr, ds): (f64, f64, bool) = conn
            .query_row(
                concat!(
                    "SELECT initialrating, currentrating, defaultschedule FROM players ",
                    "WHERE name = ?1"
                ),
                &["player2"],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(ir, 980.0);
        assert_eq!(cr, 980.0);
        assert_eq!(ds, false);
        let np: i64 = conn
            .query_row("SELECT COUNT(*) FROM players", [], |row| row.get(0))
            .unwrap();
        assert_eq!(np, 2);
    }
}

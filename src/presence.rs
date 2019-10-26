///! Presence overview page
use std::collections::HashMap;

use actix_web::Responder;
use askama::Template;
use rusqlite::{params, NO_PARAMS};

use crate::models::{PresencePlayer, Round, RoundsByMonth};
use crate::{get_today, CommonTemplate, Error, Result};

#[derive(Template)]
#[template(path = "presence.html")]
struct PresenceTemplate {
    today: String,
    rounds: RoundsByMonth,
    players: Vec<PresencePlayer>,
}
impl CommonTemplate for PresenceTemplate {}

pub(crate) fn presence(conn: &rusqlite::Connection) -> Result<impl Responder> {
    let today = get_today();
    presence_internal(conn, today)
}

fn presence_internal(conn: &rusqlite::Connection, today: String) -> Result<PresenceTemplate> {
    let mut stmt =
        conn.prepare("SELECT id, date, extra FROM rounds WHERE date >= ?1 ORDER BY date")?;
    let rounds: Vec<Round> = stmt
        .query_map(params![today], |row| {
            Ok(Round {
                id: row.get(0)?,
                date: row.get(1)?,
                extra: row.get(2)?,
            })
        })?
        .collect::<rusqlite::Result<_>>()?;
    let round_id_to_idx: HashMap<i32, usize> = rounds
        .iter()
        .enumerate()
        .map(|(idx, round)| (round.id, idx))
        .collect();
    let mut stmt = conn
        .prepare("SELECT id, name, defaultschedule FROM players ORDER BY currentrating DESC, id")?;
    let mut players: Vec<PresencePlayer> = stmt
        .query_map(NO_PARAMS, |row| {
            Ok(PresencePlayer {
                id: row.get(0)?,
                name: row.get(1)?,
                default: row.get(2)?,
                presences: vec![None; rounds.len()],
            })
        })?
        .collect::<rusqlite::Result<_>>()?;
    let player_id_to_idx: HashMap<i32, usize> = players
        .iter()
        .enumerate()
        .map(|(idx, player)| (player.id, idx))
        .collect();
    let mut stmt = conn.prepare("SELECT player, \"when\", schedule FROM presence")?;
    stmt.query_and_then(NO_PARAMS, |row| {
        let player_id: i32 = row.get(0)?;
        let round_id: i32 = row.get(1)?;
        let schedule: bool = row.get(2)?;
        if let Some(round_idx) = round_id_to_idx.get(&round_id).cloned() {
            let player_idx = player_id_to_idx
                .get(&player_id)
                .cloned()
                .ok_or(Error::Inconsistency("missing player"))?;
            let presence = &mut players[player_idx].presences[round_idx];
            if presence.is_some() {
                return Err(Error::Inconsistency("duplicate presence"));
            }
            *presence = Some(schedule);
        }
        Ok(())
    })?
    .collect::<Result<()>>()?;
    Ok(PresenceTemplate {
        today,
        rounds: RoundsByMonth(rounds),
        players,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::ensure_schema;

    #[test]
    fn presence_overview_0() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        ensure_schema(&conn).unwrap();
        let pt = presence_internal(&conn, "2019-10-06".into()).unwrap();
        assert_eq!(pt.today, "2019-10-06");
        assert!(pt.rounds.0.is_empty());
        assert!(pt.players.is_empty());
    }
}

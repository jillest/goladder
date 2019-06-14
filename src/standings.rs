use std::collections::HashMap;

use actix_web::Responder;
use askama::Template;
use rusqlite::NO_PARAMS;

use gorating::Rating;

use crate::models::{Colour, GameResult, OneSidedGame, Round, StandingsPlayer};
use crate::Result;

#[derive(Template)]
#[template(path = "standings.html")]
struct StandingsTemplate {
    today: String,
    rounds: Vec<Round>,
    players: Vec<StandingsPlayer>,
    games: i64,
    white_wins: i64,
    black_wins: i64,
    jigo: i64,
    forfeit: i64,
}

pub(crate) fn standings(conn: &rusqlite::Connection) -> Result<impl Responder> {
    let today = time::now().strftime("%Y-%m-%d").unwrap().to_string();
    let mut stmt = conn.prepare("SELECT id FROM players ORDER BY initialrating DESC, id")?;
    let original_indices: HashMap<i32, usize> = stmt
        .query_map(NO_PARAMS, |row| {
            let id: i32 = row.get(0)?;
            Ok(id)
        })?
        .enumerate()
        .map(|(idx, res_id)| Ok((res_id?, idx + 1)))
        .collect::<rusqlite::Result<_>>()?;
    let mut stmt = conn
        .prepare(
            concat!("SELECT p.id, p.name, p.initialrating, p.currentrating, COUNT(g.id), ",
            "COUNT((p.id = g.black AND g.result IN ('BlackWins', 'BlackWinsByDefault')) OR (p.id = g.white AND g.result IN ('WhiteWins', 'WhiteWinsByDefault')) OR NULL), ",
            "COUNT(g.result = 'Jigo' OR NULL) ",
            "FROM players p ",
            "LEFT OUTER JOIN games g ON (p.id = g.black OR p.id = g.white) AND g.result IS NOT NULL ",
            "GROUP BY p.id ORDER BY p.currentrating DESC, p.id"),
        )?;
    let mut players: Vec<StandingsPlayer> = stmt
        .query_map(NO_PARAMS, |row| {
            let id: i32 = row.get(0)?;
            let name: String = row.get(1)?;
            let initialrating = Rating::new(row.get(2)?);
            let currentrating = Rating::new(row.get(3)?);
            let games: i64 = row.get(4)?;
            let wins: i64 = row.get(5)?;
            let jigos: i64 = row.get(6)?;
            let score = wins as f64 + 0.5 * jigos as f64;
            Ok(StandingsPlayer {
                id,
                original_index: original_indices[&id],
                name,
                initialrating,
                currentrating,
                results: Vec::new(),
                score,
                games,
            })
        })?
        .collect::<rusqlite::Result<_>>()?;
    let mut rounds = Vec::<Round>::new();
    let (mut games, mut white_wins, mut black_wins, mut jigo, mut forfeit) = (0, 0, 0, 0, 0);
    {
        let mut players_by_id: HashMap<i32, (usize, &mut StandingsPlayer)> = players
            .iter_mut()
            .enumerate()
            .map(|t| (t.1.id, t))
            .collect();
        let mut stmt = conn.prepare(concat!(
            "SELECT r.id, r.date, g.id, g.white, g.black, g.handicap, g.result ",
            "FROM rounds r, games g ",
            "WHERE g.played = r.id AND g.result IS NOT NULL ",
            "ORDER BY r.date, g.id"
        ))?;
        stmt.query_map(NO_PARAMS, |row| {
            let round_id: i32 = row.get(0)?;
            let round_date: String = row.get(1)?;
            let game_id: i32 = row.get(2)?;
            let white_id: i32 = row.get(3)?;
            let black_id: i32 = row.get(4)?;
            let handicap: f64 = row.get(5)?;
            let result: GameResult = row.get(6)?;
            if rounds.last().map(|r| r.id) != Some(round_id) {
                rounds.push(Round {
                    id: round_id,
                    date: round_date,
                });
            }
            let black_place = players_by_id.get(&black_id).map(|t| t.0 + 1).unwrap_or(0);
            let white_place = players_by_id.get(&white_id).map(|t| t.0 + 1).unwrap_or(0);
            if let Some((_, black)) = players_by_id.get_mut(&black_id) {
                let osg = OneSidedGame {
                    id: game_id,
                    colour: Colour::Black,
                    other_place: white_place,
                    handicap: handicap,
                    result: result.seen_from_black(),
                };
                while black.results.len() < rounds.len() {
                    black.results.push(Vec::new());
                }
                black.results.last_mut().unwrap().push(osg);
            }
            if let Some((_, white)) = players_by_id.get_mut(&white_id) {
                let osg = OneSidedGame {
                    id: game_id,
                    colour: Colour::White,
                    other_place: black_place,
                    handicap: handicap,
                    result: result.seen_from_white(),
                };
                while white.results.len() < rounds.len() {
                    white.results.push(Vec::new());
                }
                white.results.last_mut().unwrap().push(osg);
            }
            games += 1;
            match result {
                GameResult::WhiteWins => white_wins += 1,
                GameResult::BlackWins => black_wins += 1,
                GameResult::Jigo => jigo += 1,
                GameResult::WhiteWinsByDefault
                | GameResult::BlackWinsByDefault
                | GameResult::BothLose => forfeit += 1,
            };
            Ok(())
        })?
        .collect::<rusqlite::Result<()>>()?;
    }
    for player in players.iter_mut() {
        while player.results.len() < rounds.len() {
            player.results.push(Vec::new());
        }
    }
    Ok(StandingsTemplate {
        today,
        rounds,
        players,
        games,
        white_wins,
        black_wins,
        jigo,
        forfeit,
    })
}

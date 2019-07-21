use std::collections::HashMap;

use actix_web::Responder;
use askama::Template;
use rusqlite::NO_PARAMS;

use gorating::Rating;

use crate::models::{Colour, GameResult, OneSidedGame, Round, StandingsPlayer};
use crate::{get_today, CommonTemplate, Result};

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
impl CommonTemplate for StandingsTemplate {}

pub(crate) fn standings(conn: &rusqlite::Connection) -> Result<impl Responder> {
    let today = get_today();
    standings_internal(conn, today)
}

fn standings_internal(conn: &rusqlite::Connection, today: String) -> Result<StandingsTemplate> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::ensure_schema;
    use crate::models::OneSidedGameResult;

    #[test]
    fn calc_standings_0() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        ensure_schema(&conn).unwrap();
        let st = standings_internal(&conn, "2019-06-18".into()).unwrap();
        assert_eq!(st.today, "2019-06-18");
        assert!(st.rounds.is_empty());
        assert!(st.players.is_empty());
        assert_eq!(st.games, 0);
        assert_eq!(st.white_wins, 0);
        assert_eq!(st.black_wins, 0);
        assert_eq!(st.jigo, 0);
        assert_eq!(st.forfeit, 0);
    }

    #[test]
    fn calc_standings_1() {
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
                    NO_PARAMS,
                )
                .unwrap();
            trans
                .execute(
                    concat!(
                        "INSERT INTO rounds (id, \"date\") VALUES ",
                        "(99, '2019-06-17');",
                    ),
                    NO_PARAMS,
                )
                .unwrap();
            trans
                .execute(
                    concat!(
                        "INSERT INTO games (id, played, white, black, result) VALUES ",
                        "(33, 99, 41, 42, 'WhiteWins');"
                    ),
                    NO_PARAMS,
                )
                .unwrap();
            trans.commit().unwrap();
        }
        let st = standings_internal(&conn, "2019-06-18".into()).unwrap();
        assert_eq!(st.today, "2019-06-18");
        assert_eq!(st.rounds.len(), 1);
        let round = &st.rounds[0];
        assert_eq!(round.id, 99);
        assert_eq!(round.date, "2019-06-17");
        assert_eq!(st.players.len(), 2);
        {
            let p1 = &st.players[0];
            assert_eq!(p1.id, 41);
            assert_eq!(p1.name, "player1");
            assert_eq!(p1.initialrating.0, 1000.0);
            assert_eq!(p1.currentrating.0, 1020.0);
            assert_eq!(p1.results.len(), 1);
            let p1r1 = &p1.results[0];
            assert_eq!(p1r1.len(), 1);
            let p1r1r = &p1r1[0];
            assert_eq!(p1r1r.id, 33);
            assert_eq!(p1r1r.colour, Colour::White);
            assert_eq!(p1r1r.result, OneSidedGameResult::Win);
            assert_eq!(p1.score, 1.0);
            assert_eq!(p1.games, 1);
        }
        {
            let p2 = &st.players[1];
            assert_eq!(p2.id, 42);
            assert_eq!(p2.name, "player2");
            assert_eq!(p2.initialrating.0, 1000.0);
            assert_eq!(p2.currentrating.0, 980.0);
            assert_eq!(p2.results.len(), 1);
            let p2r1 = &p2.results[0];
            assert_eq!(p2r1.len(), 1);
            let p2r1r = &p2r1[0];
            assert_eq!(p2r1r.id, 33);
            assert_eq!(p2r1r.colour, Colour::Black);
            assert_eq!(p2r1r.result, OneSidedGameResult::Lose);
            assert_eq!(p2.score, 0.0);
            assert_eq!(p2.games, 1);
        }
        assert_eq!(st.games, 1);
        assert_eq!(st.white_wins, 1);
        assert_eq!(st.black_wins, 0);
        assert_eq!(st.jigo, 0);
        assert_eq!(st.forfeit, 0);
    }
}

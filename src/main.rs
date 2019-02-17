#[macro_use]
extern crate postgres_derive;

use std::collections::HashMap;
use std::collections::HashSet;
use std::str::FromStr;
use std::sync::Arc;

use actix_web::{http, server, App, Form, HttpResponse, Path, Responder, State};
use askama::Template;

mod db;
mod models;

use crate::models::{FormattableGameResult, Game, GameResult, Player, Round, RoundPresence};

struct AppState {
    dbpool: Arc<db::Pool>,
}

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    rounds: Vec<Round>,
}

fn index(state: State<AppState>) -> impl Responder {
    let conn = state.dbpool.get().unwrap();
    let rows = conn
        .query("SELECT id, date::TEXT FROM rounds ORDER BY date", &[])
        .unwrap();
    let rounds: Vec<Round> = rows
        .iter()
        .map(|row| {
            let id: i32 = row.get(0);
            let date: String = row.get(1);
            Round { id, date }
        })
        .collect();
    IndexTemplate { rounds }
}

#[derive(Template)]
#[template(path = "schedule_round.html")]
struct ScheduleRoundTemplate {
    round: Round,
    games: Vec<Game>,
    presences: Vec<RoundPresence>,
}

fn schedule_round((params, state): (Path<(i32,)>, State<AppState>)) -> impl Responder {
    let round_id = params.0;
    let conn = state.dbpool.get().unwrap();
    let rows = conn
        .query("SELECT date::TEXT FROM rounds WHERE id=$1;", &[&round_id])
        .unwrap();
    let round_date = rows.iter().next().map_or_else(
        || "??".to_owned(),
        |row| {
            let date: String = row.get(0);
            date
        },
    );
    let round = Round {
        id: round_id,
        date: round_date,
    };
    let rows = conn.query("SELECT g.id, pw.id, pw.name, pw.currentrating, pb.id, pb.name, pb.currentrating, g.handicap, g.result FROM players pw, players pb, games g WHERE pw.id = g.white AND pb.id = g.black AND g.played = $1 ORDER BY g.id;", &[&round_id]).unwrap();
    let games: Vec<Game> = rows
        .iter()
        .map(|row| {
            let id: i32 = row.get(0);
            let white_id: i32 = row.get(1);
            let white: String = row.get(2);
            let white_rating: f64 = row.get(3);
            let black_id: i32 = row.get(4);
            let black: String = row.get(5);
            let black_rating: f64 = row.get(6);
            let handicap: f64 = row.get(7);
            let result: Option<GameResult> = row.get(8);
            Game {
                id,
                white: Player {
                    id: white_id,
                    name: white,
                    rating: white_rating,
                },
                black: Player {
                    id: black_id,
                    name: black,
                    rating: black_rating,
                },
                handicap,
                result: FormattableGameResult(result),
            }
        })
        .collect();
    let pairedplayers = {
        let mut pairedplayers = HashSet::with_capacity(2 * games.len());
        for game in &games {
            pairedplayers.insert(game.white.id);
            pairedplayers.insert(game.black.id);
        }
        pairedplayers
    };
    let rows = conn.query("SELECT pl.id, pl.name, pl.currentrating, COALESCE(pr.schedule, pl.defaultschedule) FROM players pl LEFT OUTER JOIN presence pr ON pl.id = pr.player AND pr.\"when\" = $1;",
        &[&round_id]).unwrap();
    let presences: Vec<RoundPresence> = rows
        .iter()
        .filter_map(|row| {
            let player_id: i32 = row.get(0);
            let name: String = row.get(1);
            let rating: f64 = row.get(2);
            let schedule: bool = row.get(3);
            if !schedule || pairedplayers.contains(&player_id) {
                None
            } else {
                Some(RoundPresence {
                    player: Player {
                        id: player_id,
                        name,
                        rating,
                    },
                    schedule,
                })
            }
        })
        .collect();
    ScheduleRoundTemplate {
        round,
        games,
        presences,
    }
}

fn unpair_games(
    conn: &db::Connection,
    round_id: i32,
    unpair_game_ids: &[i32],
) -> postgres::Result<()> {
    if unpair_game_ids.len() == 0 {
        return Ok(());
    }
    let n = conn.execute(
        "DELETE FROM games WHERE played = $1 AND id = ANY ($2);",
        &[&round_id, &unpair_game_ids],
    )?;
    eprintln!("deleted {} game(s)", n);
    Ok(())
}

fn pair_players(conn: &db::Connection, round_id: i32, player_ids: &[i32]) -> postgres::Result<()> {
    if player_ids.len() == 0 {
        return Ok(());
    }
    let rows = conn.query("SELECT g.white, g.black FROM games g, rounds r WHERE g.played = r.id AND (g.white = ANY ($1) OR g.black = ANY ($1)) AND g.result IS NOT NULL ORDER BY r.date DESC, r.id DESC;",
        &[&player_ids])?;
    let mut played = vec![0; player_ids.len()];
    let mut weights = vec![vec![0; player_ids.len()]; player_ids.len()];
    for row in &rows {
        let white: i32 = row.get(0);
        let black: i32 = row.get(1);
        let white_idx_opt = player_ids.binary_search(&white).ok();
        let black_idx_opt = player_ids.binary_search(&black).ok();
        if let Some(white_idx) = white_idx_opt {
            played[white_idx] += 1;
        }
        if let Some(black_idx) = black_idx_opt {
            played[black_idx] += 1;
        }
        if let (Some(white_idx), Some(black_idx)) = (white_idx_opt, black_idx_opt) {
            let w = &mut weights[white_idx][black_idx];
            let val = i32::min(played[white_idx], played[black_idx]);
            if *w == 0 || *w > val {
                *w = val;
                weights[black_idx][white_idx] = *w;
            }
        }
    }
    let rows = conn.query(
        "SELECT currentrating FROM players WHERE id = ANY ($1) ORDER BY id;",
        &[&player_ids],
    )?;
    let ratings: Vec<f64> = rows.iter().map(|row| row.get(0)).collect();
    assert_eq!(ratings.len(), player_ids.len());
    const DONT_MATCH_AGAIN_PARAM: f64 = 1000.0;
    const DONT_MATCH_AGAIN_DECAY: f64 = 2.0;
    const RATING_POINTS_PER_CLASS: f64 = 50.0;
    for (i, w_row) in weights.iter_mut().enumerate() {
        for (j, w) in w_row.iter_mut().enumerate() {
            if i == j {
                continue;
            }
            if *w > 0 {
                *w = (DONT_MATCH_AGAIN_PARAM * (-(*w - 1) as f64 / DONT_MATCH_AGAIN_DECAY).exp())
                    as i32;
            }
            let diff = (ratings[i] - ratings[j]) / RATING_POINTS_PER_CLASS;
            *w += (diff * diff) as i32;
        }
    }
    eprintln!("weights = {:?}", weights);
    let matching = weightedmatch::weightedmatch(weights, weightedmatch::MINIMIZE);
    eprintln!("matching = {:?}", matching);
    let trans = conn.transaction()?;
    let statement = conn
        .prepare("INSERT INTO games (played, white, black, handicap) VALUES ($1, $2, $3, $4);")?;
    for (player, opponent) in matching.iter().skip(1).map(|idx| idx - 1).enumerate() {
        if (ratings[player], player) < (ratings[opponent], opponent) {
            continue;
        }
        eprintln!(
            "schedule: {}({}) vs {}({})",
            player_ids[player], ratings[player], player_ids[opponent], ratings[opponent]
        );
        let diff = ratings[player] - ratings[opponent];
        let handicap: f64 = if diff < 50.0 {
            0.0
        } else {
            let unrounded = 0.5 + diff / 100.0;
            (unrounded * 2.0).round() * 0.5
        };
        statement.execute(&[
            &round_id,
            &player_ids[player],
            &player_ids[opponent],
            &handicap,
        ])?;
    }
    trans.commit()
}

fn schedule_round_run(
    (pathparams, state, params): (Path<(i32,)>, State<AppState>, Form<HashMap<String, String>>),
) -> actix_web::Result<HttpResponse> {
    let round_id = pathparams.0;
    let mut player_ids: Vec<i32> = params
        .0
        .keys()
        .filter_map(|s| {
            if s.starts_with("p") {
                i32::from_str(&s[1..]).ok()
            } else {
                None
            }
        })
        .collect();
    player_ids.sort_unstable();
    let unpair_game_ids: Vec<i32> = params
        .0
        .keys()
        .filter_map(|s| {
            if s.starts_with("unpair") {
                i32::from_str(&s[6..]).ok()
            } else {
                None
            }
        })
        .collect();
    let conn = state.dbpool.get().unwrap();
    unpair_games(&conn, round_id, &unpair_game_ids).unwrap();
    pair_players(&conn, round_id, &player_ids).unwrap();
    Ok(HttpResponse::Found()
        .header(http::header::LOCATION, format!("/schedule/{}", round_id))
        .finish())
}

#[derive(Template)]
#[template(path = "add_round.html")]
struct AddRoundTemplate {
    defaultdate: String,
}

fn add_round(state: State<AppState>) -> impl Responder {
    let conn = state.dbpool.get().unwrap();
    let rows = conn
        .query(
            "SELECT (COALESCE(MAX(date) + '1 week'::interval, NOW()))::date::TEXT FROM rounds;",
            &[],
        )
        .unwrap();
    let defaultdate: String = {
        let row = rows.iter().next().unwrap();
        row.get(0)
    };
    AddRoundTemplate { defaultdate }
}

fn add_round_run(
    (state, params): (State<AppState>, Form<HashMap<String, String>>),
) -> actix_web::Result<HttpResponse> {
    let date = &params.0["date"];
    let conn = state.dbpool.get().unwrap();
    conn.execute(
        "INSERT INTO rounds (date) VALUES ($1::TEXT::date);",
        &[&date],
    )
    .unwrap();
    Ok(HttpResponse::Found()
        .header(http::header::LOCATION, "/")
        .finish())
}

#[derive(Template)]
#[template(path = "players.html")]
struct PlayersTemplate {
    players: Vec<Player>,
}

fn players(state: State<AppState>) -> impl Responder {
    let conn = state.dbpool.get().unwrap();
    let rows = conn
        .query(
            "SELECT id, name, currentrating FROM players ORDER BY currentrating DESC",
            &[],
        )
        .unwrap();
    let players: Vec<Player> = rows
        .iter()
        .map(|row| {
            let id: i32 = row.get(0);
            let name: String = row.get(1);
            let rating: f64 = row.get(2);
            Player { id, name, rating }
        })
        .collect();
    PlayersTemplate { players }
}

fn main() {
    let dbpool = Arc::new(db::create_pool());
    server::new(move || {
        App::with_state(AppState {
            dbpool: dbpool.clone(),
        })
        .route("/", http::Method::GET, index)
        .resource("/schedule/{round}", |r| {
            r.method(http::Method::GET).with(schedule_round);
            r.method(http::Method::POST).with(schedule_round_run)
        })
        .resource("/add_round", |r| {
            r.method(http::Method::GET).with(add_round);
            r.method(http::Method::POST).with(add_round_run)
        })
        .route("/players", http::Method::GET, players)
    })
    .bind("127.0.0.1:8080")
    .unwrap()
    .run();
}

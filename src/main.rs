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
mod update_ratings;

use crate::models::{
    FormattableGameResult, Game, GameResult, Player, PlayerPresence, PlayerRoundPresence, Round,
    RoundPresence,
};

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

fn modify_games(
    conn: &db::Connection,
    round_id: i32,
    game_actions: &[(i32, &str)],
) -> postgres::Result<()> {
    if game_actions.len() == 0 {
        return Ok(());
    }
    let unpair_game_ids: Vec<i32> = game_actions
        .iter()
        .filter_map(
            |&(id, action)| {
                if action == "delete" {
                    Some(id)
                } else {
                    None
                }
            },
        )
        .collect();
    let trans = conn.transaction()?;
    let n = trans.execute(
        "DELETE FROM games WHERE played = $1 AND id = ANY ($2);",
        &[&round_id, &unpair_game_ids],
    )?;
    eprintln!("deleted {} game(s)", n);
    let mut ratings_changed = n > 0;
    let statement = trans.prepare("UPDATE games SET result = $1 WHERE played = $2 AND id = $3;")?;
    for &(id, action) in game_actions {
        let result = match action {
            "delete" => continue,
            "None" => None,
            "BlackWins" => Some(GameResult::BlackWins),
            "WhiteWins" => Some(GameResult::WhiteWins),
            "Jigo" => Some(GameResult::Jigo),
            "BlackWinsByDefault" => Some(GameResult::BlackWinsByDefault),
            "WhiteWinsByDefault" => Some(GameResult::WhiteWinsByDefault),
            "BothLose" => Some(GameResult::BothLose),
            _ => {
                eprintln!("unknown game action: {}", action);
                continue;
            }
        };
        if statement.execute(&[&result, &round_id, &id])? > 0 {
            ratings_changed = true;
        }
    }
    if ratings_changed {
        update_ratings::update_ratings(&trans)?;
    }
    trans.commit()
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
    let game_actions: Vec<(i32, &str)> = params
        .0
        .iter()
        .filter_map(|(k, v)| {
            if k.starts_with("action") && v != "" {
                i32::from_str(&k[6..]).ok().map(|id| (id, v.as_str()))
            } else {
                None
            }
        })
        .collect();
    let conn = state.dbpool.get().unwrap();
    modify_games(&conn, round_id, &game_actions).unwrap();
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

#[derive(Template)]
#[template(path = "edit_player.html")]
struct EditPlayerTemplate {
    is_new: bool,
    player: Player,
    presence: PlayerPresence,
}

fn add_player(_state: State<AppState>) -> impl Responder {
    EditPlayerTemplate {
        is_new: true,
        player: Player {
            id: 0,
            name: "".into(),
            rating: 1100.0,
        },
        presence: PlayerPresence {
            default: true,
            rounds: vec![],
        },
    }
}

fn update_player_presence(
    trans: &postgres::transaction::Transaction,
    player_id: i32,
    params: &HashMap<String, String>,
) -> postgres::Result<()> {
    for (k, v) in params.iter() {
        if !k.starts_with("schedule") {
            continue;
        }
        let round_id = match i32::from_str(&k[8..]) {
            Ok(x) => x,
            Err(_) => continue,
        };
        match v.as_str() {
            "default" => {
                trans.execute(
                    "DELETE FROM presence WHERE player = $1 AND \"when\" = $2;",
                    &[&player_id, &round_id],
                )?;
            }
            "true" => {
                trans.execute(
                    concat!(
                        "INSERT INTO presence (player, \"when\", schedule) VALUES ($1, $2, $3) ",
                        "ON CONFLICT (player, \"when\") DO UPDATE SET schedule = $3;"
                    ),
                    &[&player_id, &round_id, &true],
                )?;
            }
            "false" => {
                trans.execute(
                    concat!(
                        "INSERT INTO presence (player, \"when\", schedule) VALUES ($1, $2, $3) ",
                        "ON CONFLICT (player, \"when\") DO UPDATE SET schedule = $3;"
                    ),
                    &[&player_id, &round_id, &false],
                )?;
            }
            _ => eprintln!("bad presence update \"{}\"", v),
        }
    }
    Ok(())
}

fn add_player_save(
    (state, params): (State<AppState>, Form<HashMap<String, String>>),
) -> actix_web::Result<HttpResponse> {
    let name = &params.0["name"];
    let initialrating = f64::from_str(&params.0["initialrating"]).unwrap();
    let defaultschedule = params.0.get("defaultschedule").is_some();
    let conn = state.dbpool.get().unwrap();
    conn.execute(
        "INSERT INTO players (name, initialrating, currentrating, defaultschedule) VALUES ($1, $2, $2, $3);",
        &[&name, &initialrating, &defaultschedule],
    )
    .unwrap();
    Ok(HttpResponse::Found()
        .header(http::header::LOCATION, "/players")
        .finish())
}

fn edit_player((params, state): (Path<(i32,)>, State<AppState>)) -> impl Responder {
    let player_id = params.0;
    let conn = state.dbpool.get().unwrap();
    let rows = conn
        .query(
            "SELECT r.id, r.date::TEXT, pr.schedule FROM rounds r LEFT OUTER JOIN presence pr ON r.id = pr.\"when\" AND pr.player = $1 ORDER BY r.date",
            &[&player_id],
        )
        .unwrap();
    let rpresence: Vec<_> = rows
        .iter()
        .map(|row| PlayerRoundPresence {
            round_id: row.get(0),
            round_date: row.get(1),
            schedule: row.get(2),
        })
        .collect();
    let rows = conn
        .query(
            "SELECT id, name, initialrating, defaultschedule FROM players WHERE id = $1",
            &[&player_id],
        )
        .unwrap();
    let (player, presence) = {
        let row = rows.iter().next().unwrap();
        let id: i32 = row.get(0);
        let name: String = row.get(1);
        let rating: f64 = row.get(2);
        let default: bool = row.get(3);
        (
            Player { id, name, rating },
            PlayerPresence {
                default,
                rounds: rpresence,
            },
        )
    };
    EditPlayerTemplate {
        is_new: false,
        player,
        presence,
    }
}

fn edit_player_save(
    (pathparams, state, params): (Path<(i32,)>, State<AppState>, Form<HashMap<String, String>>),
) -> actix_web::Result<HttpResponse> {
    let player_id = pathparams.0;
    let name = &params.0["name"];
    let initialrating = f64::from_str(&params.0["initialrating"]).unwrap();
    let defaultschedule = params.0.get("defaultschedule").is_some();
    let conn = state.dbpool.get().unwrap();
    let trans = conn.transaction().unwrap();
    trans
        .execute(
            "UPDATE players SET name = $1, initialrating = $2, defaultschedule = $3 WHERE id = $4;",
            &[&name, &initialrating, &defaultschedule, &player_id],
        )
        .unwrap();
    update_player_presence(&trans, player_id, &params.0).unwrap();
    update_ratings::update_ratings(&trans).unwrap();
    trans.commit().unwrap();
    Ok(HttpResponse::Found()
        .header(http::header::LOCATION, "/players")
        .finish())
}

fn main() {
    let dburl = std::env::args().nth(1).unwrap_or_else(|| {
        eprintln!("Need a database URL such as \"postgresql://jilles@%2Ftmp/goladder\"");
        std::process::exit(2);
    });
    let dbpool = Arc::new(db::create_pool(&dburl));
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
        .resource("/add_player", |r| {
            r.method(http::Method::GET).with(add_player);
            r.method(http::Method::POST).with(add_player_save)
        })
        .resource("/player/{id}", |r| {
            r.method(http::Method::GET).with(edit_player);
            r.method(http::Method::POST).with(edit_player_save)
        })
    })
    .bind("127.0.0.1:8080")
    .unwrap()
    .run();
}

use std::borrow::Cow;
use std::collections::HashMap;
use std::collections::HashSet;
use std::str::FromStr;
use std::sync::Arc;

use actix_web::{http, server, App, Body, Form, HttpResponse, Path, Responder, State};
use askama::Template;
use rusqlite::types::ToSql;
use rusqlite::{OptionalExtension, NO_PARAMS};
use rust_embed::RustEmbed;

mod db;
mod models;
mod update_ratings;

use crate::models::{
    FormattableGameResult, Game, GameResult, Player, PlayerPresence, PlayerRoundPresence, Round,
    RoundPresence, StandingsPlayer,
};

struct AppState {
    dbpool: Arc<db::Pool>,
}

#[derive(Debug)]
enum Error {
    Database(rusqlite::Error),
    DatabasePool(r2d2::Error),
    BadParam(&'static str),
    ActixWeb(actix_web::Error),
}

impl From<rusqlite::Error> for Error {
    fn from(e: rusqlite::Error) -> Self {
        Error::Database(e)
    }
}

impl From<r2d2::Error> for Error {
    fn from(e: r2d2::Error) -> Self {
        Error::DatabasePool(e)
    }
}

impl From<actix_web::Error> for Error {
    fn from(e: actix_web::Error) -> Self {
        Error::ActixWeb(e)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::Database(inner) => write!(f, "Database: {}", inner),
            Error::DatabasePool(inner) => write!(f, "Database pool: {}", inner),
            Error::BadParam(inner) => write!(f, "Invalid parameter: {}", inner),
            Error::ActixWeb(inner) => write!(f, "{}", inner),
        }
    }
}

type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(RustEmbed)]
#[folder = "static/"]
struct StaticAsset;

fn guess_content_type(path: &str) -> &'static str {
    if path.ends_with(".html") {
        "text/html"
    } else if path.ends_with(".png") {
        "image/png"
    } else if path.ends_with(".jpg") {
        "image/jpeg"
    } else if path.ends_with(".js") {
        "application/javascript"
    } else if path.ends_with(".css") {
        "text/css"
    } else {
        "application/octet-stream"
    }
}

fn static_asset((params, _state): (Path<(String,)>, State<AppState>)) -> HttpResponse {
    let path = &params.0;
    match StaticAsset::get(path) {
        Some(content) => {
            let body: Body = match content {
                Cow::Borrowed(bytes) => bytes.into(),
                Cow::Owned(bytes) => bytes.into(),
            };
            HttpResponse::Ok()
                .content_type(guess_content_type(path))
                .body(body)
        }
        None => HttpResponse::NotFound().body("404 Not found"),
    }
}

#[derive(Template)]
#[template(path = "error.html")]
struct ErrorTemplate {
    class: &'static str,
    message: String,
}

fn transform_error(error: Error) -> actix_web::Error {
    let template = match error {
        Error::Database(ref inner) => ErrorTemplate {
            class: "Database",
            message: inner.to_string(),
        },
        Error::DatabasePool(ref inner) => ErrorTemplate {
            class: "Database pool",
            message: inner.to_string(),
        },
        Error::BadParam(inner) => ErrorTemplate {
            class: "Bad parameter",
            message: inner.to_string(),
        },
        Error::ActixWeb(inner) => return inner,
    };
    let resp = match template.render() {
        Ok(html) => HttpResponse::InternalServerError()
            .content_type("text/html")
            .body(html),
        Err(_) => HttpResponse::InternalServerError()
            .content_type("text/plain")
            .body("An error occurred, and another error occurred while trying to display it."),
    };
    actix_web::error::InternalError::from_response(error, resp).into()
}

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    rounds: Vec<Round>,
}

fn index(state: State<AppState>) -> Result<impl Responder> {
    let conn = state.dbpool.get()?;
    let mut stmt = conn.prepare("SELECT id, CAST(date AS TEXT) FROM rounds ORDER BY date")?;
    let rounds: Vec<Round> = stmt
        .query_map(NO_PARAMS, |row| {
            let id: i32 = row.get(0)?;
            let date: String = row.get(1)?;
            Ok(Round { id, date })
        })?
        .collect::<rusqlite::Result<_>>()?;
    Ok(IndexTemplate { rounds })
}

#[derive(Template)]
#[template(path = "schedule_round.html")]
struct ScheduleRoundTemplate {
    round: Round,
    games: Vec<Game>,
    presences: Vec<RoundPresence>,
}

fn schedule_round((params, state): (Path<(i32,)>, State<AppState>)) -> Result<impl Responder> {
    let round_id = params.0;
    let conn = state.dbpool.get()?;
    let round_date = conn
        .query_row(
            "SELECT CAST(date AS TEXT) FROM rounds WHERE id=?1",
            &[&round_id],
            |row| {
                let date: String = row.get(0)?;
                Ok(date)
            },
        )
        .optional()?
        .unwrap_or("??".to_owned());
    let round = Round {
        id: round_id,
        date: round_date,
    };
    let mut stmt = conn.prepare("SELECT g.id, pw.id, pw.name, pw.currentrating, pb.id, pb.name, pb.currentrating, g.handicap, g.result FROM players pw, players pb, games g WHERE pw.id = g.white AND pb.id = g.black AND g.played = ?1 ORDER BY g.id")?;
    let games: Vec<Game> = stmt
        .query_map(&[&round_id], |row| {
            let id: i32 = row.get(0)?;
            let white_id: i32 = row.get(1)?;
            let white: String = row.get(2)?;
            let white_rating: f64 = row.get(3)?;
            let black_id: i32 = row.get(4)?;
            let black: String = row.get(5)?;
            let black_rating: f64 = row.get(6)?;
            let handicap: f64 = row.get(7)?;
            let result: Option<GameResult> = row.get(8)?;
            Ok(Game {
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
            })
        })?
        .collect::<rusqlite::Result<_>>()?;
    let pairedplayers = {
        let mut pairedplayers = HashSet::with_capacity(2 * games.len());
        for game in &games {
            pairedplayers.insert(game.white.id);
            pairedplayers.insert(game.black.id);
        }
        pairedplayers
    };
    let mut stmt = conn.prepare("SELECT pl.id, pl.name, pl.currentrating, COALESCE(pr.schedule, pl.defaultschedule) FROM players pl LEFT OUTER JOIN presence pr ON pl.id = pr.player AND pr.\"when\" = ?1",
        )?;
    let presences: Vec<RoundPresence> = stmt
        .query_map(&[&round_id], |row| {
            let player_id: i32 = row.get(0)?;
            let name: String = row.get(1)?;
            let rating: f64 = row.get(2)?;
            let schedule: bool = row.get(3)?;
            Ok(if !schedule || pairedplayers.contains(&player_id) {
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
            })
        })?
        .filter_map(Result::transpose)
        .collect::<rusqlite::Result<_>>()?;
    Ok(ScheduleRoundTemplate {
        round,
        games,
        presences,
    })
}

fn modify_games(
    conn: &mut db::Connection,
    round_id: i32,
    game_actions: &[(i32, &str)],
) -> Result<()> {
    if game_actions.len() == 0 {
        return Ok(());
    }
    let trans = conn.transaction()?;
    let mut ratings_changed = false;
    let mut d_stmt = trans.prepare("DELETE FROM games WHERE played = ?1 AND id = ?2")?;
    let mut u_stmt = trans.prepare("UPDATE games SET result = ?1 WHERE played = ?2 AND id = ?3")?;
    for &(id, action) in game_actions {
        let result: Option<&str> = match action {
            "delete" => {
                if d_stmt.execute(&[round_id, id])? > 0 {
                    ratings_changed = true;
                }
                continue;
            }
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
        }
        .map(GameResult::to_str);
        if u_stmt.execute::<&[&dyn ToSql]>(&[&result, &round_id, &id])? > 0 {
            ratings_changed = true;
        }
    }
    drop(u_stmt);
    drop(d_stmt);
    if ratings_changed {
        update_ratings::update_ratings(&trans)?;
    }
    trans.commit()?;
    Ok(())
}

fn pair_players(conn: &mut db::Connection, round_id: i32, player_ids: &[i32]) -> Result<()> {
    if player_ids.len() == 0 {
        return Ok(());
    }
    let mut played = vec![0; player_ids.len()];
    let mut weights = vec![vec![0; player_ids.len()]; player_ids.len()];
    let trans = conn.transaction()?;
    {
        let mut stmt = trans.prepare("SELECT g.white, g.black FROM games g, rounds r WHERE g.played = r.id AND g.result IS NOT NULL ORDER BY r.date DESC, r.id DESC")?;
        struct GameRow {
            white: i32,
            black: i32,
        }
        let rows: Vec<GameRow> = stmt
            .query_map(NO_PARAMS, |row| {
                Ok(GameRow {
                    white: row.get(0)?,
                    black: row.get(1)?,
                })
            })?
            .collect::<rusqlite::Result<_>>()?;
        for row in &rows {
            let white_idx_opt = player_ids.binary_search(&row.white).ok();
            let black_idx_opt = player_ids.binary_search(&row.black).ok();
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
    }
    let ratings: Vec<f64>;
    {
        let mut stmt = trans.prepare("SELECT id, currentrating FROM players ORDER BY id")?;
        ratings = stmt
            .query_map(NO_PARAMS, |row| {
                let id: i32 = row.get(0)?;
                let rating: f64 = row.get(1)?;
                Ok(player_ids.binary_search(&id).ok().map(|_| rating))
            })?
            .filter_map(Result::transpose)
            .collect::<rusqlite::Result<_>>()?;
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
                    *w = (DONT_MATCH_AGAIN_PARAM
                        * (-(*w - 1) as f64 / DONT_MATCH_AGAIN_DECAY).exp())
                        as i32;
                }
                let diff = (ratings[i] - ratings[j]) / RATING_POINTS_PER_CLASS;
                *w += (diff * diff) as i32;
            }
        }
    }
    eprintln!("weights = {:?}", weights);
    let matching = weightedmatch::weightedmatch(weights, weightedmatch::MINIMIZE);
    eprintln!("matching = {:?}", matching);
    {
        let mut stmt = trans.prepare(
            "INSERT INTO games (played, white, black, handicap) VALUES (?1, ?2, ?3, ?4)",
        )?;
        for (player, opponent) in matching.iter().skip(1).map(|idx| idx - 1).enumerate() {
            if (ratings[player], player) < (ratings[opponent], opponent) {
                continue;
            }
            eprintln!(
                "schedule: {}({}) vs {}({})",
                player_ids[player], ratings[player], player_ids[opponent], ratings[opponent]
            );
            let diff = ratings[player] - ratings[opponent];
            let handicap = update_ratings::RATINGS.calculate_handicap(diff);
            stmt.execute::<&[&dyn ToSql]>(&[
                &round_id,
                &player_ids[player],
                &player_ids[opponent],
                &handicap,
            ])?;
        }
    }
    trans.commit()?;
    Ok(())
}

fn schedule_round_run(
    (pathparams, state, params): (Path<(i32,)>, State<AppState>, Form<HashMap<String, String>>),
) -> Result<HttpResponse> {
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
    let mut conn = state.dbpool.get()?;
    modify_games(&mut conn, round_id, &game_actions)?;
    pair_players(&mut conn, round_id, &player_ids)?;
    Ok(HttpResponse::Found()
        .header(http::header::LOCATION, format!("/schedule/{}", round_id))
        .finish())
}

#[derive(Template)]
#[template(path = "add_round.html")]
struct AddRoundTemplate {
    defaultdate: String,
}

fn add_round(state: State<AppState>) -> Result<impl Responder> {
    let conn = state.dbpool.get()?;
    let defaultdate: String = conn.query_row(
        "SELECT COALESCE(date(MAX(rounds.date), '+7 days'), date('now')) FROM rounds",
        NO_PARAMS,
        |row| row.get(0),
    )?;
    Ok(AddRoundTemplate { defaultdate })
}

fn add_round_run(
    (state, params): (State<AppState>, Form<HashMap<String, String>>),
) -> Result<HttpResponse> {
    let date = &params.0["date"];
    let conn = state.dbpool.get()?;
    conn.execute("INSERT INTO rounds (date) VALUES (?1)", &[date])?;
    Ok(HttpResponse::Found()
        .header(http::header::LOCATION, "/")
        .finish())
}

#[derive(Template)]
#[template(path = "players.html")]
struct PlayersTemplate {
    players: Vec<Player>,
}

fn players(state: State<AppState>) -> Result<impl Responder> {
    let conn = state.dbpool.get()?;
    let mut stmt =
        conn.prepare("SELECT id, name, currentrating FROM players ORDER BY currentrating DESC")?;
    let players: Vec<Player> = stmt
        .query_map(NO_PARAMS, |row| {
            let id: i32 = row.get(0)?;
            let name: String = row.get(1)?;
            let rating: f64 = row.get(2)?;
            Ok(Player { id, name, rating })
        })?
        .collect::<rusqlite::Result<_>>()?;
    Ok(PlayersTemplate { players })
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
    trans: &rusqlite::Transaction,
    player_id: i32,
    params: &HashMap<String, String>,
) -> Result<()> {
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
                trans.execute::<&[&dyn ToSql]>(
                    "DELETE FROM presence WHERE player = ?1 AND \"when\" = ?2",
                    &[&player_id, &round_id],
                )?;
            }
            "true" => {
                if trans.execute::<&[&dyn ToSql]>(
                    "UPDATE presence SET schedule = ?3 WHERE player = ?1 AND \"when\" = ?2",
                    &[&player_id, &round_id, &true],
                )? == 0
                {
                    trans.execute::<&[&dyn ToSql]>(
                        "INSERT INTO presence (player, \"when\", schedule) VALUES (?1, ?2, ?3)",
                        &[&player_id, &round_id, &true],
                    )?;
                }
            }
            "false" => {
                if trans.execute::<&[&dyn ToSql]>(
                    "UPDATE presence SET schedule = ?3 WHERE player = ?1 AND \"when\" = ?2",
                    &[&player_id, &round_id, &false],
                )? == 0
                {
                    trans.execute::<&[&dyn ToSql]>(
                        "INSERT INTO presence (player, \"when\", schedule) VALUES (?1, ?2, ?3)",
                        &[&player_id, &round_id, &false],
                    )?;
                }
            }
            _ => eprintln!("bad presence update \"{}\"", v),
        }
    }
    Ok(())
}

fn add_player_save(
    (state, params): (State<AppState>, Form<HashMap<String, String>>),
) -> Result<HttpResponse> {
    let name = &params.0["name"];
    let initialrating =
        f64::from_str(&params.0["initialrating"]).map_err(|_| Error::BadParam("initialrating"))?;
    let defaultschedule = params.0.get("defaultschedule").is_some();
    let conn = state.dbpool.get()?;
    conn.execute::<&[&dyn ToSql]>(
        "INSERT INTO players (name, initialrating, currentrating, defaultschedule) VALUES (?1, ?2, ?2, ?3)",
        &[&name, &initialrating, &defaultschedule],
    )?;
    Ok(HttpResponse::Found()
        .header(http::header::LOCATION, "/players")
        .finish())
}

fn edit_player((params, state): (Path<(i32,)>, State<AppState>)) -> Result<impl Responder> {
    let player_id = params.0;
    let conn = state.dbpool.get()?;
    let mut stmt = conn
        .prepare(
            "SELECT r.id, CAST(r.date AS TEXT), pr.schedule FROM rounds r LEFT OUTER JOIN presence pr ON r.id = pr.\"when\" AND pr.player = ?1 ORDER BY r.date",
        )?;
    let rpresence: Vec<_> = stmt
        .query_map(&[&player_id], |row| {
            Ok(PlayerRoundPresence {
                round_id: row.get(0)?,
                round_date: row.get(1)?,
                schedule: row.get(2)?,
            })
        })?
        .collect::<rusqlite::Result<_>>()?;
    let (player, presence) = conn.query_row(
        "SELECT id, name, initialrating, defaultschedule FROM players WHERE id = ?1",
        &[&player_id],
        |row| {
            let id: i32 = row.get(0)?;
            let name: String = row.get(1)?;
            let rating: f64 = row.get(2)?;
            let default: bool = row.get(3)?;
            Ok((
                Player { id, name, rating },
                PlayerPresence {
                    default,
                    rounds: rpresence,
                },
            ))
        },
    )?;
    Ok(EditPlayerTemplate {
        is_new: false,
        player,
        presence,
    })
}

fn edit_player_save(
    (pathparams, state, params): (Path<(i32,)>, State<AppState>, Form<HashMap<String, String>>),
) -> Result<HttpResponse> {
    let player_id = pathparams.0;
    let name = &params.0["name"];
    let initialrating =
        f64::from_str(&params.0["initialrating"]).map_err(|_| Error::BadParam("initialrating"))?;
    let defaultschedule = params.0.get("defaultschedule").is_some();
    let mut conn = state.dbpool.get()?;
    let trans = conn.transaction()?;
    trans.execute::<&[&dyn ToSql]>(
        "UPDATE players SET name = ?1, initialrating = ?2, defaultschedule = ?3 WHERE id = ?4",
        &[&name, &initialrating, &defaultschedule, &player_id],
    )?;
    update_player_presence(&trans, player_id, &params.0)?;
    update_ratings::update_ratings(&trans)?;
    trans.commit()?;
    Ok(HttpResponse::Found()
        .header(http::header::LOCATION, "/players")
        .finish())
}

#[derive(Template)]
#[template(path = "standings.html")]
struct StandingsTemplate {
    players: Vec<StandingsPlayer>,
    games: i64,
    white_wins: i64,
    black_wins: i64,
    jigo: i64,
    forfeit: i64,
}

fn standings(state: State<AppState>) -> Result<impl Responder> {
    let conn = state.dbpool.get()?;
    let mut stmt = conn
        .prepare(
            concat!("SELECT p.id, p.name, p.initialrating, p.currentrating, COUNT(g.id), ",
            "COUNT((p.id = g.black AND g.result IN ('BlackWins', 'BlackWinsByDefault')) OR (p.id = g.white AND g.result IN ('WhiteWins', 'WhiteWinsByDefault')) OR NULL), ",
            "COUNT(g.result = 'Jigo' OR NULL) ",
            "FROM players p ",
            "LEFT OUTER JOIN games g ON (p.id = g.black OR p.id = g.white) AND g.result IS NOT NULL ",
            "GROUP BY p.id ORDER BY p.currentrating DESC, p.id"),
        )?;
    let players: Vec<StandingsPlayer> = stmt
        .query_map(NO_PARAMS, |row| {
            let id: i32 = row.get(0)?;
            let name: String = row.get(1)?;
            let initialrating: f64 = row.get(2)?;
            let currentrating: f64 = row.get(3)?;
            let games: i64 = row.get(4)?;
            let wins: i64 = row.get(5)?;
            let jigos: i64 = row.get(6)?;
            let score = wins as f64 + 0.5 * jigos as f64;
            Ok(StandingsPlayer {
                id,
                name,
                initialrating,
                currentrating,
                score,
                games,
            })
        })?
        .collect::<rusqlite::Result<_>>()?;
    let (games, white_wins, black_wins, jigo, forfeit) =
        conn.query_row(
            concat!("SELECT COUNT(result), COUNT(result = 'WhiteWins' OR NULL), ",
        "COUNT(result = 'BlackWins' OR NULL), COUNT(result = 'Jigo' OR NULL), ",
        "COUNT(result IN ('WhiteWinsByDefault', 'BlackWinsByDefault', 'BothLose') OR NULL) ",
        "FROM games"),
            NO_PARAMS,
            |row| {
                let games: i64 = row.get(0)?;
                let white_wins: i64 = row.get(1)?;
                let black_wins: i64 = row.get(2)?;
                let jigo: i64 = row.get(3)?;
                let forfeit: i64 = row.get(4)?;
                Ok((games, white_wins, black_wins, jigo, forfeit))
            },
        )?;
    Ok(StandingsTemplate {
        players,
        games,
        white_wins,
        black_wins,
        jigo,
        forfeit,
    })
}

fn handle_error<T, U>(f: impl Fn(T) -> Result<U>) -> impl Fn(T) -> actix_web::Result<U> {
    move |x| f(x).map_err(transform_error)
}

fn main() {
    let dbpath = std::env::args_os().nth(1).unwrap_or_else(|| {
        eprintln!("Need a database pathname such as \"goladder.db\"");
        std::process::exit(2);
    });
    let dbpool = Arc::new(db::create_pool(&dbpath));
    server::new(move || {
        App::with_state(AppState {
            dbpool: dbpool.clone(),
        })
        .route("/", http::Method::GET, handle_error(index))
        .resource("/schedule/{round}", |r| {
            r.method(http::Method::GET)
                .with(handle_error(schedule_round));
            r.method(http::Method::POST)
                .with(handle_error(schedule_round_run))
        })
        .resource("/add_round", |r| {
            r.method(http::Method::GET).with(handle_error(add_round));
            r.method(http::Method::POST)
                .with(handle_error(add_round_run))
        })
        .route("/players", http::Method::GET, handle_error(players))
        .resource("/add_player", |r| {
            r.method(http::Method::GET).with(add_player);
            r.method(http::Method::POST)
                .with(handle_error(add_player_save))
        })
        .resource("/player/{id}", |r| {
            r.method(http::Method::GET).with(handle_error(edit_player));
            r.method(http::Method::POST)
                .with(handle_error(edit_player_save))
        })
        .route("/standings", http::Method::GET, handle_error(standings))
        .resource("/static/{path:.*}", |r| {
            r.method(http::Method::GET).with(static_asset)
        })
    })
    .bind("127.0.0.1:8080")
    .unwrap()
    .run();
}

#[macro_use]
extern crate postgres_derive;

use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;

use actix_web::{http, server, App, Form, HttpResponse, Path, Responder, State};
use askama::Template;
use postgres::{self, to_sql_checked};

mod db;

struct AppState {
    dbpool: Arc<db::Pool>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
struct PlayerId(u32);

#[derive(Debug)]
struct Player {
    id: PlayerId,
    name: String,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, FromSql, ToSql)]
#[postgres(name = "gameresult")]
enum GameResult {
    WhiteWins,
    BlackWins,
    Jigo,
    WhiteWinsByDefault,
    BlackWinsByDefault,
    BothLose,
}

#[derive(Debug)]
struct FormattableGameResult(Option<GameResult>);

impl std::fmt::Display for FormattableGameResult {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        let s = match self.0 {
            None => "?-?",
            Some(GameResult::WhiteWins) => "1-0",
            Some(GameResult::BlackWins) => "0-1",
            Some(GameResult::Jigo) => "½-½",
            Some(GameResult::WhiteWinsByDefault) => "1-0!",
            Some(GameResult::BlackWinsByDefault) => "0-1!",
            Some(GameResult::BothLose) => "0-0",
        };
        write!(formatter, "{}", s)
    }
}

#[derive(Debug)]
struct Game {
    white: Player,
    black: Player,
    result: FormattableGameResult,
}

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    games: Vec<Game>,
}

fn index(state: State<AppState>) -> impl Responder {
    let conn = state.dbpool.get().unwrap();
    let rows = conn.query("SELECT pw.name, pb.name, g.result FROM players pw, players pb, games g WHERE pw.id = g.white AND pb.id = g.black ORDER BY g.id;", &[]).unwrap();
    let games: Vec<Game> = rows.iter().map(|row| {
        let white: String = row.get(0);
        let black: String = row.get(1);
        let result: Option<GameResult> = row.get(2);
        Game {
            white: Player { id: PlayerId(0), name: white },
            black: Player { id: PlayerId(0), name: black },
            result: FormattableGameResult(result),
        }
    }).collect();
    IndexTemplate { games }
}

#[derive(Debug)]
struct Round {
    id: i32,
    date: String,
}

#[derive(Debug)]
struct Presence {
    player_id: i32,
    name: String,
    schedule: bool,
}

#[derive(Template)]
#[template(path = "schedule_round.html")]
struct ScheduleRoundTemplate {
    round: Round,
    presences: Vec<Presence>,
}

fn schedule_round((params, state): (Path<(i32,)>, State<AppState>)) -> impl Responder {
    let round_id = params.0;
    let conn = state.dbpool.get().unwrap();
    let rows = conn.query("SELECT date::TEXT FROM rounds WHERE id=$1;", &[&round_id]).unwrap();
    let round_date = rows.iter().next().map_or_else(|| "??".to_owned(), |row| {
        let date: String = row.get(0);
        date
    });
    let round = Round { id: round_id, date: round_date };
    let rows = conn.query("SELECT pl.id, pl.name, pr.schedule FROM players pl, presence pr WHERE pl.id = pr.player AND \"when\"=$1;",
        &[&round_id]).unwrap();
    let presences: Vec<Presence> = rows.iter().map(|row| {
        let player_id: i32 = row.get(0);
        let name: String = row.get(1);
        let schedule: bool = row.get(2);
        Presence {
            player_id,
            name,
            schedule
        }
    }).collect();
    ScheduleRoundTemplate { round, presences }
}

fn schedule_round_run((pathparams, state, params): (Path<(i32,)>, State<AppState>, Form<HashMap<String, String>>)) -> actix_web::Result<HttpResponse> {
    let round_id = pathparams.0;
    let mut player_ids: Vec<i32> = params.0.keys().filter_map(|s| if s.starts_with("p") {
        i32::from_str(&s[1..]).ok()
    } else {
        None
    }).collect();
    player_ids.sort_unstable();
    let player_ids = player_ids;
    let conn = state.dbpool.get().unwrap();
    let rows = conn.query("SELECT white, black FROM games WHERE (white = ANY ($1) OR black = ANY ($1)) AND result IS NOT NULL ORDER BY played DESC;",
        &[&player_ids]).unwrap();
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
    let rows = conn.query("SELECT currentrating FROM players WHERE id = ANY ($1) ORDER BY id;",
        &[&player_ids]).unwrap();
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
                *w = (-DONT_MATCH_AGAIN_PARAM * (-(*w - 1) as f64 / DONT_MATCH_AGAIN_DECAY).exp()) as i32;
            }
            let diff = (ratings[i] - ratings[j]) / RATING_POINTS_PER_CLASS;
            *w -= (diff * diff) as i32;
        }
    }
    eprintln!("weights = {:?}", weights);
    Ok(HttpResponse::build(http::StatusCode::OK)
        .content_type("text/plain")
        .body("Scheduled OK"))
}

fn main() {
    let dbpool = Arc::new(db::create_pool());
    server::new(
        move || App::with_state(AppState { dbpool: dbpool.clone() })
            .route("/", http::Method::GET, index)
            .resource("/schedule/{round}", |r| r.method(http::Method::GET).with(schedule_round))
            .resource("/schedule/{round}/run", |r| r.method(http::Method::POST).with(schedule_round_run)))
        .bind("127.0.0.1:8080").unwrap()
        .run();
}

#[macro_use]
extern crate postgres_derive;

use std::sync::Arc;

use actix_web::{http, server, App, Path, Responder, State};
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

fn main() {
    let dbpool = Arc::new(db::create_pool());
    server::new(
        move || App::with_state(AppState { dbpool: dbpool.clone() })
            .route("/", http::Method::GET, index)
            .resource("/schedule/{round}", |r| r.method(http::Method::GET).with(schedule_round)))
        .bind("127.0.0.1:8080").unwrap()
        .run();
}

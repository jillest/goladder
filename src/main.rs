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

fn main() {
    let dbpool = Arc::new(db::create_pool());
    server::new(
        move || App::with_state(AppState { dbpool: dbpool.clone() })
            .route("/index.html", http::Method::GET, index))
        .bind("127.0.0.1:8080").unwrap()
        .run();
}

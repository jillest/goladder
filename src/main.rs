use std::sync::Arc;

use actix_web::{http, server, App, Path, Responder, State};
use askama::Template;

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

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum GameResult {
    Unknown,
    WhiteWins,
    BlackWins,
    WhiteWinsByDefault,
    BlackWinsByDefault,
    BothLose,
}

impl std::fmt::Display for GameResult {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        let s = match self {
            GameResult::Unknown => "?-?",
            GameResult::WhiteWins => "1-0",
            GameResult::BlackWins => "0-1",
            GameResult::WhiteWinsByDefault => "1-0!",
            GameResult::BlackWinsByDefault => "0-1!",
            GameResult::BothLose => "0-0",
        };
        write!(formatter, "{}", s)
    }
}

#[derive(Debug)]
struct Game {
    white: Player,
    black: Player,
    result: GameResult,
}

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    games: Vec<Game>,
}

fn index(state: State<AppState>) -> impl Responder {
    let conn = state.dbpool.get().unwrap();
    let rows = conn.query("SELECT pw.name, pb.name, g.result FROM players pw, players pb, games g WHERE pw.id = g.white AND pb.id = g.black;", &[]).unwrap();
    let games: Vec<Game> = rows.iter().map(|row| {
        let white: String = row.get(0);
        let black: String = row.get(1);
        Game {
            white: Player { id: PlayerId(0), name: white },
            black: Player { id: PlayerId(0), name: black },
            result: GameResult::Unknown,
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

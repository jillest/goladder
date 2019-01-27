use std::collections::HashMap;

use actix_web::{http, server, App, Path, Responder};
use askama::Template;

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

fn index(info: Path<()>) -> impl Responder {
    let p1 = Player { id: PlayerId(1), name: "p1".into() };
    let p2 = Player { id: PlayerId(2), name: "p2".into() };
    let p3 = Player { id: PlayerId(3), name: "p3".into() };
    let p4 = Player { id: PlayerId(4), name: "p4".into() };
    let games = vec![
        Game { white: p1, black: p2, result: GameResult::Unknown },
        Game { white: p3, black: p4, result: GameResult::Unknown },
    ];
    IndexTemplate { games }
}

fn main() {
    server::new(
        || App::new()
            .route("/index.html", http::Method::GET, index))
        .bind("127.0.0.1:8080").unwrap()
        .run();
}

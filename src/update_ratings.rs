use std::collections::HashMap;

use gorating::RatingSystem;
use postgres::transaction::Transaction;

use crate::models::GameResult;

static RATINGS: RatingSystem = RatingSystem {
    epsilon: 0.016,
    min_rating: -400.0,
};

pub fn update_ratings(trans: &Transaction) -> postgres::Result<()> {
    let rows = trans.query("SELECT id, initialrating FROM players;", &[])?;
    let mut ratings: HashMap<i32, f64> = rows.iter().map(|row| (row.get(0), row.get(1))).collect();
    let rows = trans.query(
        "SELECT g.white, g.black, g.handicap, g.boardsize, g.result FROM games g, rounds r WHERE g.played = r.id AND g.result IS NOT NULL ORDER BY r.date;",
        &[],
    )?;
    for row in rows.iter() {
        let white: i32 = row.get(0);
        let black: i32 = row.get(1);
        let handicap: f64 = row.get(2);
        let _boardsize: i16 = row.get(3);
        let result: GameResult = row.get(4);
        let wresult = match result {
            GameResult::WhiteWins => 1.0,
            GameResult::BlackWins => 0.0,
            GameResult::Jigo => 0.5,
            _ => continue,
        };
        let bresult = 1.0 - wresult;
        let wadj = RATINGS.rating_adjustment(ratings[&white], ratings[&black], -handicap, wresult);
        let badj = RATINGS.rating_adjustment(ratings[&black], ratings[&white], handicap, bresult);
        let wr = ratings.get_mut(&white).expect("game's player not found");
        *wr = f64::max(*wr + wadj, RATINGS.min_rating);
        let br = ratings.get_mut(&black).expect("game's player not found");
        *br = f64::max(*br + badj, RATINGS.min_rating);
    }
    let statement = trans.prepare("UPDATE players SET currentrating = $2 WHERE id = $1;")?;
    for (id, rating) in ratings.iter() {
        statement.execute(&[&id, &rating])?;
    }
    Ok(())
}

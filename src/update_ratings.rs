use std::cell::Cell;
use std::collections::HashMap;

use gorating::{Rating, RatingSystem};
use rusqlite::types::ToSql;
use rusqlite::{Transaction, NO_PARAMS};

use crate::models::GameResult;

pub static RATINGS: RatingSystem = RatingSystem {
    epsilon: 0.016,
    min_rating: Rating(-400.0),
    max_drop: 100.0,
};

struct PendingRating {
    rating: Rating,
    pending: Cell<f64>,
}

impl PendingRating {
    fn new(rating: f64) -> Self {
        PendingRating {
            rating: Rating(rating),
            pending: Cell::new(0.0),
        }
    }
}

fn apply_pending_changes(ratings: &mut HashMap<i32, PendingRating>) {
    for pr in ratings.values_mut() {
        let adj = f64::max(pr.pending.replace(0.0), -RATINGS.max_drop);
        pr.rating = RATINGS.adjust_rating(pr.rating, adj);
    }
}

pub fn update_ratings(trans: &Transaction) -> rusqlite::Result<()> {
    let mut stmt = trans.prepare("SELECT id, initialrating FROM players")?;
    let mut ratings: HashMap<i32, PendingRating> = stmt
        .query_map(NO_PARAMS, |row| {
            Ok((row.get(0)?, PendingRating::new(row.get(1)?)))
        })?
        .collect::<rusqlite::Result<_>>()?;
    let mut last_round = None;
    let mut stmt = trans.prepare(
        "SELECT g.white, g.black, g.handicap, g.boardsize, g.result, r.id FROM games g, rounds r WHERE g.played = r.id AND g.result IS NOT NULL ORDER BY r.date"
    )?;
    stmt.query_map(NO_PARAMS, |row| {
        let white: i32 = row.get(0)?;
        let black: i32 = row.get(1)?;
        let handicap: f64 = row.get(2)?;
        let _boardsize: i16 = row.get(3)?;
        let result: GameResult = row.get(4)?;
        let round: i32 = row.get(5)?;
        if Some(round) != last_round {
            last_round = Some(round);
            apply_pending_changes(&mut ratings);
        }
        let wresult = match result {
            GameResult::WhiteWins => 1.0,
            GameResult::BlackWins => 0.0,
            GameResult::Jigo => 0.5,
            _ => return Ok(()),
        };
        let bresult = 1.0 - wresult;
        let wpr = &ratings[&white];
        let bpr = &ratings[&black];
        let wadj = RATINGS.rating_adjustment(wpr.rating, bpr.rating, -handicap, wresult);
        let badj = RATINGS.rating_adjustment(bpr.rating, wpr.rating, handicap, bresult);
        wpr.pending.set(wpr.pending.get() + wadj);
        bpr.pending.set(bpr.pending.get() + badj);
        Ok(())
    })?
    .collect::<rusqlite::Result<()>>()?;
    apply_pending_changes(&mut ratings);
    let mut statement = trans.prepare("UPDATE players SET currentrating = ?2 WHERE id = ?1")?;
    for (id, rating) in ratings.iter() {
        statement.execute::<&[&dyn ToSql]>(&[&id, &rating.rating.0])?;
    }
    Ok(())
}

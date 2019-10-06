use std::str::FromStr;

use gorating::{Handicap, Rating};

#[derive(Debug)]
pub struct Player {
    pub id: i32,
    pub name: String,
    pub rating: Rating,
}

#[derive(Debug)]
pub struct PlayerRoundPresence {
    pub round_id: i32,
    pub round_date: String,
    pub schedule: Option<bool>,
}

impl PlayerRoundPresence {
    pub fn is_default(&self) -> bool {
        self.schedule.is_none()
    }
    pub fn is_present(&self) -> bool {
        self.schedule == Some(true)
    }
    pub fn is_absent(&self) -> bool {
        self.schedule == Some(false)
    }
}

#[derive(Debug)]
pub struct PlayerPresence {
    pub default: bool,
    pub rounds: Vec<PlayerRoundPresence>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum GameResult {
    WhiteWins,
    BlackWins,
    Jigo,
    WhiteWinsByDefault,
    BlackWinsByDefault,
    BothLose,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum OneSidedGameResult {
    Win,
    Lose,
    Jigo,
    WinByDefault,
    LoseByDefault,
}

#[derive(Debug)]
pub struct BadGameResult;

impl std::fmt::Display for BadGameResult {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "invalid game result")
    }
}

impl std::error::Error for BadGameResult {}

impl FromStr for GameResult {
    type Err = BadGameResult;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "WhiteWins" => Ok(GameResult::WhiteWins),
            "BlackWins" => Ok(GameResult::BlackWins),
            "Jigo" => Ok(GameResult::Jigo),
            "WhiteWinsByDefault" => Ok(GameResult::WhiteWinsByDefault),
            "BlackWinsByDefault" => Ok(GameResult::BlackWinsByDefault),
            "BothLose" => Ok(GameResult::BothLose),
            _ => Err(BadGameResult),
        }
    }
}

impl GameResult {
    pub fn to_str(self) -> &'static str {
        match self {
            GameResult::WhiteWins => "WhiteWins",
            GameResult::BlackWins => "BlackWins",
            GameResult::Jigo => "Jigo",
            GameResult::WhiteWinsByDefault => "WhiteWinsByDefault",
            GameResult::BlackWinsByDefault => "BlackWinsByDefault",
            GameResult::BothLose => "BothLose",
        }
    }

    pub fn seen_from_white(self) -> OneSidedGameResult {
        match self {
            GameResult::WhiteWins => OneSidedGameResult::Win,
            GameResult::BlackWins => OneSidedGameResult::Lose,
            GameResult::Jigo => OneSidedGameResult::Jigo,
            GameResult::WhiteWinsByDefault => OneSidedGameResult::WinByDefault,
            GameResult::BlackWinsByDefault => OneSidedGameResult::LoseByDefault,
            GameResult::BothLose => OneSidedGameResult::LoseByDefault,
        }
    }

    pub fn seen_from_black(self) -> OneSidedGameResult {
        match self {
            GameResult::WhiteWins => OneSidedGameResult::Lose,
            GameResult::BlackWins => OneSidedGameResult::Win,
            GameResult::Jigo => OneSidedGameResult::Jigo,
            GameResult::WhiteWinsByDefault => OneSidedGameResult::LoseByDefault,
            GameResult::BlackWinsByDefault => OneSidedGameResult::WinByDefault,
            GameResult::BothLose => OneSidedGameResult::LoseByDefault,
        }
    }
}

#[derive(Debug)]
pub struct FormattableGameResult(pub Option<GameResult>);

impl FormattableGameResult {
    pub fn is_unknown(&self) -> bool {
        self.0.is_none()
    }
}

impl std::fmt::Display for FormattableGameResult {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        let s = match self.0 {
            None => "?-?",
            Some(GameResult::WhiteWins) => "0-1",
            Some(GameResult::BlackWins) => "1-0",
            Some(GameResult::Jigo) => "½-½",
            Some(GameResult::WhiteWinsByDefault) => "0-1!",
            Some(GameResult::BlackWinsByDefault) => "1-0!",
            Some(GameResult::BothLose) => "0-0",
        };
        write!(formatter, "{}", s)
    }
}

impl std::fmt::Display for OneSidedGameResult {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        let s = match self {
            OneSidedGameResult::Win => "+",
            OneSidedGameResult::Lose => "−",
            OneSidedGameResult::Jigo => "=",
            OneSidedGameResult::WinByDefault => "+!",
            OneSidedGameResult::LoseByDefault => "−!",
        };
        write!(formatter, "{}", s)
    }
}

#[derive(Debug)]
pub struct Game {
    pub id: i32,
    pub white: Player,
    pub black: Player,
    pub handicap: Handicap,
    pub result: FormattableGameResult,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Colour {
    Black,
    White,
}

impl Colour {
    fn letter(self) -> char {
        match self {
            Colour::Black => 'b',
            Colour::White => 'w',
        }
    }
}

#[derive(Debug)]
pub struct OneSidedGame {
    pub id: i32,
    pub colour: Colour,
    pub other_place: usize,
    pub handicap: f64,
    pub result: OneSidedGameResult,
}

impl std::fmt::Display for OneSidedGame {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            formatter,
            "{}/{}{}{}",
            self.other_place,
            self.colour.letter(),
            self.handicap,
            self.result
        )
    }
}

#[derive(Debug)]
pub struct Round {
    pub id: i32,
    pub date: String,
}

/// Rounds, which will be iterated over grouped by month
#[derive(Debug)]
pub struct RoundsByMonth(pub Vec<Round>);

/// All rounds in a particular month
pub struct MonthRounds<'a> {
    pub year_and_month: &'a str,
    pub rounds: &'a [Round],
}

impl<'a> IntoIterator for &'a RoundsByMonth {
    type Item = MonthRounds<'a>;
    type IntoIter = RoundsByMonthIterator<'a>;

    fn into_iter(self) -> Self::IntoIter {
        RoundsByMonthIterator(self.0.as_slice())
    }
}

pub struct RoundsByMonthIterator<'a>(&'a [Round]);

impl<'a> Iterator for RoundsByMonthIterator<'a> {
    type Item = MonthRounds<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let first_round = self.0.get(0)?;
        let year_and_month = &first_round.date[..7];
        let mut i = 1;
        while let Some(round) = self.0.get(i) {
            if year_and_month != &round.date[..7] {
                break;
            }
            i += 1;
        }
        let (now, rest) = self.0.split_at(i);
        self.0 = rest;
        Some(MonthRounds {
            year_and_month,
            rounds: now,
        })
    }
}

#[derive(Debug)]
pub struct RoundPresence {
    pub player: Player,
    pub schedule: bool,
}

pub struct StandingsPlayer {
    pub id: i32,
    pub original_index: usize,
    pub name: String,
    pub initialrating: Rating,
    pub currentrating: Rating,
    pub results: Vec<Vec<OneSidedGame>>,
    pub score: f64,
    pub games: i64,
}

pub struct PlaceDiff(isize);

impl std::fmt::Display for PlaceDiff {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            formatter,
            "{}{}",
            if self.0 < 0 { '−' } else { '+' },
            self.0.abs()
        )
    }
}

pub struct RatingDiff(f64);

impl std::fmt::Display for RatingDiff {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            formatter,
            "{}{}",
            if self.0 < 0.0 { '−' } else { '+' },
            self.0.round().abs()
        )
    }
}

impl StandingsPlayer {
    pub fn place_diff(&self, index: usize) -> PlaceDiff {
        PlaceDiff(index as isize - self.original_index as isize)
    }

    pub fn rating_diff(&self) -> RatingDiff {
        RatingDiff(self.currentrating - self.initialrating)
    }
}

pub struct PresencePlayer {
    pub id: i32,
    pub name: String,
    pub default: bool,
    pub presences: Vec<Option<bool>>,
}

impl PresencePlayer {
    pub fn format_round_presence(&self, presence: &Option<bool>) -> &'static str {
        if presence.unwrap_or(self.default) {
            "+"
        } else {
            "−"
        }
    }

    pub fn format_default_presence(&self) -> &'static str {
        self.format_round_presence(&None)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_rounds_by_month_1() {
        let rbm = &RoundsByMonth(Vec::new());
        let r: Vec<MonthRounds> = rbm.into_iter().collect();
        assert_eq!(r.len(), 0);
    }

    #[test]
    fn test_rounds_by_month_2() {
        let rbm = &RoundsByMonth(vec![
            Round {
                id: 21,
                date: "2019-06-24".into(),
            },
            Round {
                id: 22,
                date: "2019-07-01".into(),
            },
            Round {
                id: 23,
                date: "2019-07-08".into(),
            },
            Round {
                id: 24,
                date: "2019-07-15".into(),
            },
        ]);
        let r: Vec<MonthRounds> = rbm.into_iter().collect();
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].year_and_month, "2019-06");
        assert_eq!(r[0].rounds.len(), 1);
        assert_eq!(r[0].rounds[0].id, 21);
        assert_eq!(r[0].rounds[0].date, "2019-06-24");
        assert_eq!(r[1].year_and_month, "2019-07");
        assert_eq!(r[1].rounds.len(), 3);
        assert_eq!(r[1].rounds[0].id, 22);
        assert_eq!(r[1].rounds[0].date, "2019-07-01");
        assert_eq!(r[1].rounds[2].id, 24);
        assert_eq!(r[1].rounds[2].date, "2019-07-15");
    }

    #[test]
    fn test_rating_diff_1() {
        let p = StandingsPlayer {
            id: 1,
            original_index: 1,
            name: "Dummy".into(),
            initialrating: Rating::new(1000.0),
            currentrating: Rating::new(1100.3),
            results: vec![],
            score: 3.0,
            games: 5,
        };
        assert_eq!(&format!("{}", p.rating_diff()), "+100");
    }

    #[test]
    fn test_rating_diff_2() {
        let p = StandingsPlayer {
            id: 1,
            original_index: 1,
            name: "Dummy".into(),
            initialrating: Rating::new(1001.0),
            currentrating: Rating::new(990.0),
            results: vec![],
            score: 0.0,
            games: 1,
        };
        assert_eq!(&format!("{}", p.rating_diff()), "−11");
    }

    #[test]
    fn test_rating_diff_3() {
        let p = StandingsPlayer {
            id: 1,
            original_index: 1,
            name: "Dummy".into(),
            initialrating: Rating::new(1001.0),
            currentrating: Rating::new(1000.6),
            results: vec![],
            score: 0.5,
            games: 1,
        };
        assert_eq!(&format!("{}", p.rating_diff()), "−0");
    }
}

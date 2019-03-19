use std::str::FromStr;

#[derive(Debug)]
pub struct Player {
    pub id: i32,
    pub name: String,
    pub rating: f64,
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

#[derive(Debug)]
pub struct BadGameResult;

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
}

#[derive(Debug)]
pub struct FormattableGameResult(pub Option<GameResult>);

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
pub struct Game {
    pub id: i32,
    pub white: Player,
    pub black: Player,
    pub handicap: f64,
    pub result: FormattableGameResult,
}

#[derive(Debug)]
pub struct Round {
    pub id: i32,
    pub date: String,
}

#[derive(Debug)]
pub struct RoundPresence {
    pub player: Player,
    pub schedule: bool,
}

pub struct StandingsPlayer {
    pub id: i32,
    pub name: String,
    pub initialrating: f64,
    pub currentrating: f64,
    pub score: f64,
    pub games: i64,
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
    pub fn rating_diff(&self) -> RatingDiff {
        RatingDiff(self.currentrating - self.initialrating)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_rating_diff_1() {
        let p = StandingsPlayer {
            id: 1,
            name: "Dummy".into(),
            initialrating: 1000.0,
            currentrating: 1100.3,
            score: 3.0,
            games: 5,
        };
        assert_eq!(&format!("{}", p.rating_diff()), "+100");
    }

    #[test]
    fn test_rating_diff_2() {
        let p = StandingsPlayer {
            id: 1,
            name: "Dummy".into(),
            initialrating: 1001.0,
            currentrating: 990.0,
            score: 0.0,
            games: 1,
        };
        assert_eq!(&format!("{}", p.rating_diff()), "−11");
    }

    #[test]
    fn test_rating_diff_3() {
        let p = StandingsPlayer {
            id: 1,
            name: "Dummy".into(),
            initialrating: 1001.0,
            currentrating: 1000.6,
            score: 0.5,
            games: 1,
        };
        assert_eq!(&format!("{}", p.rating_diff()), "−0");
    }
}

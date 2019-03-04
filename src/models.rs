use postgres::to_sql_checked;

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

#[derive(Copy, Clone, Debug, PartialEq, Eq, FromSql, ToSql)]
#[postgres(name = "gameresult")]
pub enum GameResult {
    WhiteWins,
    BlackWins,
    Jigo,
    WhiteWinsByDefault,
    BlackWinsByDefault,
    BothLose,
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

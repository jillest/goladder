#[derive(Debug, Copy, Clone)]
pub struct Rating(pub f64);

impl std::ops::Sub<Rating> for Rating {
    type Output = f64;

    fn sub(self, other: Rating) -> f64 {
        self.0 - other.0
    }
}

impl std::fmt::Display for Rating {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.0.round().fmt(formatter)
    }
}

impl Rating {
    pub fn new(r: f64) -> Self {
        Rating(r)
    }

    pub fn rank(self) -> Rank {
        Rank::from_rating(self)
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Handicap(f64);

impl std::fmt::Display for Handicap {
    /// Format handicap value as handicap stones and komi
    /// ```
    /// let h = gorating::Handicap::new(3.5);
    /// assert_eq!(h.to_string(), "3b5");
    /// ```
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        if self.0 == 0.0 {
            write!(formatter, "0w6½")
        } else if self.0 == 1.0 {
            write!(formatter, "0b0")
        } else if self.0 == 1.5 {
            write!(formatter, "0b5")
        } else if self.0.round() == self.0 {
            write!(formatter, "{}b0", self.0)
        } else if (self.0 - 0.5).round() == self.0 - 0.5 {
            write!(formatter, "{}b5", self.0 - 0.5)
        } else {
            write!(formatter, "{}", self.0)
        }
    }
}

#[derive(Debug)]
/// Error parsing a handicap value from a string
pub struct BadHandicap;

impl std::str::FromStr for Handicap {
    type Err = BadHandicap;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Handicap::new(if s == "0w6½" || s == "0w6.5" {
            0.0
        } else if s == "0b0" || s == "0w0" {
            1.0
        } else if s == "0b5" {
            1.5
        } else if s.ends_with("b0") || s.ends_with("w0") {
            s[..s.len() - 2].parse().map_err(|_| BadHandicap)?
        } else if s.ends_with("b5") {
            let x: f64 = s[..s.len() - 2].parse().map_err(|_| BadHandicap)?;
            x + 0.5
        } else {
            s.parse().map_err(|_| BadHandicap)?
        }))
    }
}

impl Handicap {
    #[inline]
    pub fn new(x: f64) -> Self {
        Handicap(x)
    }

    #[inline]
    pub fn to_f64(self) -> f64 {
        self.0
    }
}

pub struct RatingSystem {
    pub bonus_factor: f64,
    pub min_rating: Rating,
    /// Maximum rating points that a player can lose in one tournament
    pub max_drop: f64,
}

impl RatingSystem {
    pub const fn new() -> Self {
        Self {
            bonus_factor: 0.2,
            min_rating: Rating(-900.0),
            max_drop: 100.0,
        }
    }

    /// Magnitude of the change
    fn con(&self, rating: f64) -> f64 {
        ((3300.0 - rating) / 200.0).powf(1.6)
    }

    /// Mapping function to make ratings correspond to ranks (handicap stones)
    fn beta(&self, rating: f64) -> f64 {
        -7.0 * (3300.0 - rating).ln()
    }

    pub fn rating_adjustment(
        &self,
        rating: Rating,
        other_rating: Rating,
        handicap: f64, // not a Handicap struct, may be negative
        result: f64,
    ) -> f64 {
        assert!(result >= 0.0 && result <= 1.0);
        let Rating(rating) = rating;
        let Rating(other_rating) = other_rating;
        let mut r1 = rating;
        let mut r2 = other_rating;
        // Increase black's rating depending on the handicap for the
        // win probability calculation (but not for bonus and con).
        if handicap > 0.0 {
            r1 += 100.0 * (handicap - 0.5);
        } else if handicap < 0.0 {
            r2 += 100.0 * (-handicap - 0.5);
        }
        let expected_result = 1.0 / (1.0 + (self.beta(r2) - self.beta(r1)).exp());
        let bonus = ((2300.0 - rating) / 80.0).exp().ln_1p() * self.bonus_factor;
        self.con(rating) * (result - expected_result) + bonus
    }

    pub fn adjust_rating(&self, rating: Rating, adj: f64) -> Rating {
        let Rating(rating) = rating;
        Rating(f64::max(rating + adj, self.min_rating.0))
    }

    /// Calculate the handicap for a given (positive) rating difference.
    /// ```
    /// let sys = gorating::RatingSystem::new();
    /// let h = sys.calculate_handicap(200.0);
    /// assert_eq!(h.to_f64(), 2.5);
    /// ```
    pub fn calculate_handicap(&self, rating_diff: f64) -> Handicap {
        assert!(rating_diff >= 0.0);
        Handicap::new(if rating_diff < 50.0 {
            0.0
        } else {
            let unrounded = 0.5 + rating_diff / 100.0;
            (unrounded * 2.0).round() * 0.5
        })
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Rank(f64);

impl Rank {
    /// Calculate a rank from a rating.
    /// ```
    /// use gorating::{Rank, Rating};
    /// let r = Rank::from_rating(Rating(100.0));
    /// assert_eq!(r.to_string(), "20k");
    /// ```
    pub fn from_rating(Rating(rating): Rating) -> Self {
        Self(rating)
    }
}

impl std::fmt::Display for Rank {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        let rating = self.0;
        if rating >= 2050.0 {
            write!(formatter, "{}d", (rating / 100.0).round() - 20.0)
        } else {
            write!(formatter, "{}k", 21.0 - (rating / 100.0).round())
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_handicap_display() {
        assert_eq!(Handicap::new(0.0).to_string(), "0w6½");
        assert_eq!(Handicap::new(1.0).to_string(), "0b0");
        assert_eq!(Handicap::new(1.5).to_string(), "0b5");
        assert_eq!(Handicap::new(2.0).to_string(), "2b0");
        assert_eq!(Handicap::new(3.5).to_string(), "3b5");
        assert_eq!(Handicap::new(12.5).to_string(), "12b5");
        assert_eq!(Handicap::new(13.0).to_string(), "13b0");
    }

    #[test]
    fn test_handicap_from_str_normal() {
        assert_eq!(Handicap::from_str("0w6½").unwrap().0, 0.0);
        assert_eq!(Handicap::from_str("0b0").unwrap().0, 1.0);
        assert_eq!(Handicap::from_str("0b5").unwrap().0, 1.5);
        assert_eq!(Handicap::from_str("2b0").unwrap().0, 2.0);
        assert_eq!(Handicap::from_str("2b5").unwrap().0, 2.5);
        assert_eq!(Handicap::from_str("12b5").unwrap().0, 12.5);
        assert_eq!(Handicap::from_str("13b0").unwrap().0, 13.0);
    }

    #[test]
    fn test_handicap_from_str_alternatives() {
        assert_eq!(Handicap::from_str("0w6.5").unwrap().0, 0.0);
        assert_eq!(Handicap::from_str("0").unwrap().0, 0.0);
        assert_eq!(Handicap::from_str("0w0").unwrap().0, 1.0);
        assert_eq!(Handicap::from_str("1").unwrap().0, 1.0);
        assert_eq!(Handicap::from_str("1.5").unwrap().0, 1.5);
        assert_eq!(Handicap::from_str("2w0").unwrap().0, 2.0);
        assert_eq!(Handicap::from_str("2.0").unwrap().0, 2.0);
        assert_eq!(Handicap::from_str("2.5").unwrap().0, 2.5);
    }

    #[test]
    fn test_handicap_from_str_errors() {
        assert!(Handicap::from_str("3w5").is_err());
        assert!(Handicap::from_str("3w8").is_err());
    }

    #[test]
    fn test_con() {
        let sys = RatingSystem::new();
        let con0 = sys.con(0.0);
        assert!(con0 > 88.71);
        assert!(con0 < 88.72);
        let con1450 = sys.con(1450.0);
        assert!(con1450 > 35.14);
        assert!(con1450 < 35.15);
        let con2800 = sys.con(2800.0);
        assert!(con2800 > 4.33);
        assert!(con2800 < 4.34);
        assert_eq!(sys.con(3300.0), 0.0);
    }

    #[test]
    fn test_ratings_no_epsilon_1() {
        let sys = RatingSystem {
            bonus_factor: 0.0,
            min_rating: Rating(100.0),
            max_drop: 100.0,
        };
        let adjw = sys.rating_adjustment(Rating(2400.0), Rating(2400.0), 0.0, 1.0);
        let adjl = sys.rating_adjustment(Rating(2400.0), Rating(2400.0), 0.0, 0.0);
        assert_eq!(adjl, -adjw);
        assert!(adjw > 5.54);
        assert!(adjw < 5.55);
    }

    #[test]
    fn test_ratings_no_epsilon_2() {
        let sys = RatingSystem {
            bonus_factor: 0.0,
            min_rating: Rating(100.0),
            max_drop: 100.0,
        };
        assert_eq!(
            sys.rating_adjustment(Rating(320.0), Rating(400.0), 0.0, 1.0)
                .round(),
            41.0
        );
        assert_eq!(
            sys.rating_adjustment(Rating(400.0), Rating(320.0), 0.0, 0.0)
                .round(),
            -39.0
        );
    }

    #[test]
    fn test_ratings_no_epsilon_handicap_5() {
        let sys = RatingSystem {
            bonus_factor: 0.0,
            min_rating: Rating(100.0),
            max_drop: 100.0,
        };
        assert_eq!(
            sys.rating_adjustment(Rating(1850.0), Rating(2400.0), 5.0, 1.0)
                .round(),
            16.0
        );
        assert_eq!(
            sys.rating_adjustment(Rating(2400.0), Rating(1850.0), -5.0, 0.0)
                .round(),
            -8.0
        );
    }

    // Check even games at various strengths.
    #[test]
    fn test_ratings_no_epsilon_generic_1() {
        let sys = RatingSystem {
            bonus_factor: 0.0,
            min_rating: Rating(-500.0),
            max_drop: 100.0,
        };
        let test_ratings = [-200.0, 100.0, 500.0, 1000.0, 1500.0, 2000.0, 2400.0];
        let mut optprevadj = None;
        for r in test_ratings.iter().map(|&x| Rating(x)) {
            let adjw = sys.rating_adjustment(r, r, 0.0, 1.0);
            let adjl = sys.rating_adjustment(r, r, 0.0, 0.0);
            assert!(adjw > 0.0);
            assert!(adjl < 0.0);
            assert_eq!(adjw, -adjl);
            if let Some(prevadj) = optprevadj {
                assert!(adjw <= prevadj);
            }
            optprevadj = Some(adjw);
        }
    }

    // Consider a player with rating 1750 and check the effect of games
    // on other players with a variety of ratings.
    #[test]
    fn test_ratings_no_epsilon_generic_2() {
        let sys = RatingSystem {
            bonus_factor: 0.0,
            min_rating: Rating(-500.0),
            max_drop: 100.0,
        };
        let test_ratings = [-200.0, 100.0, 500.0, 1000.0, 1500.0, 2000.0, 2400.0];
        let base = Rating(1750.0);
        let mut optprevadj = None;
        for r in test_ratings.iter().map(|&x| Rating(x)) {
            let adjw = sys.rating_adjustment(r, base, 0.0, 1.0);
            let adjl = sys.rating_adjustment(r, base, 0.0, 0.0);
            assert!(adjw > 0.0);
            assert!(adjl < 0.0);
            if let Some(prevadjw) = optprevadj {
                assert!(
                    adjw < prevadjw,
                    "adjw !< prevadjw: {} !< {}",
                    adjw,
                    prevadjw
                );
            }
            optprevadj = Some(adjw);
        }
    }

    // Consider players with a variety of ratings and check the effect of games
    // on a single player with rating 1750.
    #[test]
    fn test_ratings_no_epsilon_generic_3() {
        let sys = RatingSystem {
            bonus_factor: 0.0,
            min_rating: Rating(-500.0),
            max_drop: 100.0,
        };
        let test_ratings = [-200.0, 100.0, 500.0, 1000.0, 1500.0, 2000.0, 2400.0];
        let base = Rating(1750.0);
        let mut optprevadj = None;
        for r in test_ratings.iter().map(|&x| Rating(x)) {
            let adjw = sys.rating_adjustment(base, r, 0.0, 1.0);
            let adjl = sys.rating_adjustment(base, r, 0.0, 0.0);
            assert!(adjw > 0.0);
            assert!(adjl < 0.0);
            if let Some((prevadjw, prevadjl)) = optprevadj {
                assert!(
                    adjw > prevadjw,
                    "adjw !> prevadjw: {} !> {}",
                    adjw,
                    prevadjw
                );
                assert!(
                    adjl > prevadjl,
                    "adjl !> prevadjl: {} !> {}",
                    adjl,
                    prevadjl
                );
            }
            optprevadj = Some((adjw, adjl));
        }
    }

    // A sample even game from T191116B
    #[test]
    fn test_ratings_egd_sample_1() {
        let sys = RatingSystem::new();
        let rw = Rating(1406.791);
        let rb = Rating(1072.993);
        let adjw = sys.rating_adjustment(rw, rb, 0.0, 0.0);
        assert_eq!((adjw * 1000.0).round(), -25373.0);
        let adjb = sys.rating_adjustment(rb, rw, 0.0, 1.0);
        assert_eq!((adjb * 1000.0).round(), 38864.0);
    }

    // A sample handicap game from T191116B
    #[test]
    fn test_ratings_egd_sample_2() {
        let sys = RatingSystem::new();
        let rw = Rating(600.0);
        let rb = Rating(39.888);
        let adjw = sys.rating_adjustment(rw, rb, -3.0, 1.0);
        assert_eq!((adjw * 1000.0).round(), 24739.0);
        let adjb = sys.rating_adjustment(rb, rw, 3.0, 0.0);
        assert_eq!((adjb * 1000.0).round(), -22052.0);
    }

    fn adjust_rating(
        sys: &RatingSystem,
        r1: &mut Rating,
        r2: &mut Rating,
        handicap: f64,
        result: f64,
    ) {
        let adj1 = sys.rating_adjustment(*r1, *r2, handicap, result);
        let adj2 = sys.rating_adjustment(*r2, *r1, -handicap, 1.0 - result);
        r1.0 += adj1;
        r2.0 += adj2;
    }

    // Check convergence with 50% wins and 50% losses.
    #[test]
    fn test_ratings_convergence_1() {
        let sys = RatingSystem::new();
        let mut r1 = Rating(-200.0);
        let mut r2 = Rating(1500.0);
        for _ in 0..50 {
            adjust_rating(&sys, &mut r1, &mut r2, 0.0, 0.0);
            adjust_rating(&sys, &mut r1, &mut r2, 0.0, 1.0);
        }
        dbg!(r1);
        dbg!(r2);
        assert!(r1.0 > r2.0 - 50.0);
        assert!(r1.0 < r2.0 + 50.0);
    }

    // Check convergence with 50% wins and 50% losses.
    #[test]
    fn test_ratings_convergence_2() {
        let sys = RatingSystem::new();
        let mut r1 = Rating(1500.0);
        let mut r2 = Rating(-200.0);
        for _ in 0..50 {
            adjust_rating(&sys, &mut r1, &mut r2, 0.0, 0.0);
            adjust_rating(&sys, &mut r1, &mut r2, 0.0, 1.0);
        }
        dbg!(r1);
        dbg!(r2);
        assert!(r1.0 > r2.0 - 50.0);
        assert!(r1.0 < r2.0 + 50.0);
    }

    // Check convergence with 75% wins and 25% losses.
    #[test]
    fn test_ratings_convergence_3() {
        let sys = RatingSystem::new();
        let mut r1 = Rating(-200.0);
        let mut r2 = Rating(1500.0);
        for _ in 0..100 {
            adjust_rating(&sys, &mut r1, &mut r2, 0.0, 0.0);
            adjust_rating(&sys, &mut r1, &mut r2, 0.0, 1.0);
            adjust_rating(&sys, &mut r1, &mut r2, 0.0, 1.0);
            adjust_rating(&sys, &mut r1, &mut r2, 0.0, 1.0);
        }
        dbg!(r1);
        dbg!(r2);
        assert!(r1.0 > r2.0 + 50.0);
        assert!(r1.0 < r2.0 + 250.0);
    }

    // Check convergence with 75% wins and 25% losses.
    #[test]
    fn test_ratings_convergence_4() {
        let sys = RatingSystem::new();
        let mut r1 = Rating(1500.0);
        let mut r2 = Rating(-200.0);
        for _ in 0..100 {
            adjust_rating(&sys, &mut r1, &mut r2, 0.0, 0.0);
            adjust_rating(&sys, &mut r1, &mut r2, 0.0, 1.0);
            adjust_rating(&sys, &mut r1, &mut r2, 0.0, 1.0);
            adjust_rating(&sys, &mut r1, &mut r2, 0.0, 1.0);
        }
        dbg!(r1);
        dbg!(r2);
        assert!(r1.0 > r2.0 + 50.0);
        assert!(r1.0 < r2.0 + 250.0);
    }

    // Check convergence with 50% wins and 50% losses, 9 handicap.
    #[test]
    fn test_ratings_convergence_5() {
        let sys = RatingSystem::new();
        let mut r1 = Rating(-200.0);
        let mut r2 = Rating(1500.0);
        for _ in 0..50 {
            adjust_rating(&sys, &mut r1, &mut r2, 9.0, 0.0);
            adjust_rating(&sys, &mut r1, &mut r2, 9.0, 1.0);
        }
        dbg!(r1);
        dbg!(r2);
        assert!(r1.0 > r2.0 - 950.0);
        assert!(r1.0 < r2.0 - 800.0);
    }

    // Check convergence with 50% wins and 50% losses, 9 handicap.
    #[test]
    fn test_ratings_convergence_6() {
        let sys = RatingSystem::new();
        let mut r1 = Rating(1200.0);
        let mut r2 = Rating(1700.0);
        for _ in 0..50 {
            adjust_rating(&sys, &mut r1, &mut r2, 9.0, 0.0);
            adjust_rating(&sys, &mut r1, &mut r2, 9.0, 1.0);
        }
        dbg!(r1);
        dbg!(r2);
        assert!(r1.0 > r2.0 - 950.0);
        assert!(r1.0 < r2.0 - 800.0);
    }

    #[test]
    fn test_rank_kyu() {
        assert_eq!(Rank(51.0).to_string(), "20k");
        assert_eq!(Rank(100.0).to_string(), "20k");
        assert_eq!(Rank(149.0).to_string(), "20k");
        assert_eq!(Rank(151.0).to_string(), "19k");
        assert_eq!(Rank(-400.0).to_string(), "25k");
        assert_eq!(Rank(1100.0).to_string(), "10k");
        assert_eq!(Rank(2049.0).to_string(), "1k");
    }

    #[test]
    fn test_rank_dan() {
        assert_eq!(Rank(2051.0).to_string(), "1d");
        assert_eq!(Rank(2149.0).to_string(), "1d");
        assert_eq!(Rank(2151.0).to_string(), "2d");
        assert_eq!(Rank(2249.0).to_string(), "2d");
        assert_eq!(Rank(2749.0).to_string(), "7d");
    }
}

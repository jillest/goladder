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

pub struct RatingSystem {
    pub epsilon: f64,
    pub min_rating: Rating,
    /// Maximum rating points that a player can lose in one tournament
    pub max_drop: f64,
}

impl RatingSystem {
    pub fn new() -> Self {
        Self {
            epsilon: 0.016,
            min_rating: Rating(-900.0),
            max_drop: 100.0,
        }
    }

    /// Magnitude of the change
    fn con(&self, rating: f64) -> f64 {
        const TABLE: [f64; 27] = [
            116.0, 110.0, 105.0, 100.0, 95.0, 90.0, 85.0, 80.0, 75.0, 70.0, 65.0, 60.0, 55.0, 51.0,
            47.0, 43.0, 39.0, 35.0, 31.0, 27.0, 24.0, 21.0, 18.0, 15.0, 13.0, 11.0, 10.0,
        ];
        if rating <= 100.0 {
            TABLE[0]
        } else if rating >= 2700.0 {
            TABLE[26]
        } else {
            let h = (rating / 100.0).floor();
            let frac = rating / 100.0 - h;
            let idx = h as usize;
            TABLE[idx - 1] * (1.0 - frac) + TABLE[idx] * frac
        }
    }

    /// Factor to make ratings correspond to ranks (handicap stones)
    fn a(&self, rating: f64) -> f64 {
        if rating <= 100.0 {
            200.0
        } else if rating >= 2700.0 {
            70.0
        } else {
            205.0 - rating / 20.0
        }
    }

    pub fn rating_adjustment(
        &self,
        rating: Rating,
        other_rating: Rating,
        handicap: f64,
        result: f64,
    ) -> f64 {
        assert!(result >= 0.0 && result <= 1.0);
        let Rating(rating) = rating;
        let Rating(other_rating) = other_rating;
        let a = self.a(f64::min(rating, other_rating)
            + if handicap != 0.0 {
                100.0 * (handicap.abs() - 0.5)
            } else {
                0.0
            });
        let difference = other_rating - rating
            + if handicap > 0.0 {
                -100.0 * (handicap - 0.5)
            } else if handicap < 0.0 {
                100.0 * (-handicap - 0.5)
            } else {
                0.0
            };
        let expected_result = if difference >= 0.0 {
            1.0 / ((difference / a).exp() + 1.0)
        } else {
            1.0 - 1.0 / ((-difference / a).exp() + 1.0)
        };
        self.con(rating) * (result - expected_result + 0.5 * self.epsilon)
    }

    pub fn adjust_rating(&self, rating: Rating, adj: f64) -> Rating {
        let Rating(rating) = rating;
        Rating(f64::max(rating + adj, self.min_rating.0))
    }

    /// Calculate the handicap for a given (positive) rating difference.
    /// ```
    /// let sys = gorating::RatingSystem::new();
    /// let h = sys.calculate_handicap(200.0);
    /// assert_eq!(h, 2.5);
    /// ```
    pub fn calculate_handicap(&self, rating_diff: f64) -> f64 {
        assert!(rating_diff >= 0.0);
        if rating_diff < 50.0 {
            0.0
        } else {
            let unrounded = 0.5 + rating_diff / 100.0;
            (unrounded * 2.0).round() * 0.5
        }
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

    #[test]
    fn test_con() {
        let sys = RatingSystem::new();
        assert_eq!(sys.con(0.0), 116.0);
        assert_eq!(sys.con(100.0), 116.0);
        assert_eq!(sys.con(150.0), 113.0);
        assert_eq!(sys.con(200.0), 110.0);
        assert_eq!(sys.con(1450.0), 49.0);
        assert_eq!(sys.con(1425.0), 50.0);
        assert_eq!(sys.con(1475.0), 48.0);
        assert_eq!(sys.con(2700.0), 10.0);
        assert_eq!(sys.con(2800.0), 10.0);
    }

    #[test]
    fn test_a() {
        let sys = RatingSystem::new();
        assert_eq!(sys.a(0.0), 200.0);
        assert_eq!(sys.a(100.0), 200.0);
        assert_eq!(sys.a(200.0), 195.0);
        assert_eq!(sys.a(1400.0), 135.0);
        assert_eq!(sys.a(2700.0), 70.0);
        assert_eq!(sys.a(2800.0), 70.0);
    }

    #[test]
    fn test_ratings_no_epsilon_1() {
        let sys = RatingSystem {
            epsilon: 0.0,
            min_rating: Rating(100.0),
            max_drop: 100.0,
        };
        assert_eq!(
            sys.rating_adjustment(Rating(2400.0), Rating(2400.0), 0.0, 1.0),
            7.5
        );
        assert_eq!(
            sys.rating_adjustment(Rating(2400.0), Rating(2400.0), 0.0, 0.0),
            -7.5
        );
    }

    #[test]
    fn test_ratings_no_epsilon_2() {
        let sys = RatingSystem {
            epsilon: 0.0,
            min_rating: Rating(100.0),
            max_drop: 100.0,
        };
        assert_eq!(
            sys.rating_adjustment(Rating(320.0), Rating(400.0), 0.0, 1.0)
                .round(),
            63.0
        );
        assert_eq!(
            sys.rating_adjustment(Rating(400.0), Rating(320.0), 0.0, 0.0)
                .round(),
            -60.0
        );
    }

    #[test]
    fn test_ratings_no_epsilon_handicap_5() {
        let sys = RatingSystem {
            epsilon: 0.0,
            min_rating: Rating(100.0),
            max_drop: 100.0,
        };
        assert_eq!(
            sys.rating_adjustment(Rating(1850.0), Rating(2400.0), 5.0, 1.0)
                .round(),
            25.0
        );
        assert_eq!(
            sys.rating_adjustment(Rating(2400.0), Rating(1850.0), -5.0, 0.0)
                .round(),
            -11.0
        );
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

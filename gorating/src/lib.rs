pub struct RatingSystem {
    pub epsilon: f64,
    pub min_rating: f64,
}

impl RatingSystem {
    pub fn new() -> Self {
        Self {
            epsilon: 0.016,
            min_rating: -900.0,
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
        rating: f64,
        other_rating: f64,
        handicap: f64,
        result: f64,
    ) -> f64 {
        assert!(result >= 0.0 && result <= 1.0);
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
            min_rating: 100.0,
        };
        assert_eq!(sys.rating_adjustment(2400.0, 2400.0, 0.0, 1.0), 7.5);
        assert_eq!(sys.rating_adjustment(2400.0, 2400.0, 0.0, 0.0), -7.5);
    }

    #[test]
    fn test_ratings_no_epsilon_2() {
        let sys = RatingSystem {
            epsilon: 0.0,
            min_rating: 100.0,
        };
        assert_eq!(sys.rating_adjustment(320.0, 400.0, 0.0, 1.0).round(), 63.0);
        assert_eq!(sys.rating_adjustment(400.0, 320.0, 0.0, 0.0).round(), -60.0);
    }

    #[test]
    fn test_ratings_no_epsilon_handicap_5() {
        let sys = RatingSystem {
            epsilon: 0.0,
            min_rating: 100.0,
        };
        assert_eq!(
            sys.rating_adjustment(1850.0, 2400.0, 5.0, 1.0).round(),
            25.0
        );
        assert_eq!(
            sys.rating_adjustment(2400.0, 1850.0, -5.0, 0.0).round(),
            -11.0
        );
    }
}

use pheno::Phenotype;
use super::*;
use super::super::FitnessType;
use std::cmp::Ordering;
use rand::Rng;

/// Runs several tournaments, and selects best performing phenotypes from each tournament.
#[derive(Clone)]
pub struct TournamentSelector {
    count: usize,
    participants: usize,
}

impl TournamentSelector {
    /// Create and return a tournament selector.
    ///
    /// Such a selector runs `count / 2` tournaments, each with `participants` participants.
    /// From each tournament, the best 2 phenotypes are selected, yielding
    /// `count` parents.
    ///
    /// * `count`: must be larger than zero, a multiple of two and less than the population size.
    /// * `participants`: must be larger than zero and less than the population size.
    pub fn new(count: usize, participants: usize) -> TournamentSelector {
        TournamentSelector {
            count: count,
            participants: participants,
        }
    }
}

impl<T: Phenotype> Selector<T> for TournamentSelector {
    fn select(&self,
              population: &Vec<Box<T>>,
              fitness_type: FitnessType)
              -> Result<Parents<T>, String> {
        if self.count <= 0 || self.count % 2 != 0 || self.count * 2 >= population.len() {
            return Err(format!("Invalid parameter `count`: {}. Should be larger than zero, a \
                                multiple of two and less than half the population size.",
                               self.count));
        }
        if self.participants <= 0 || self.participants >= population.len() {
            return Err(format!("Invalid parameter `participants`: {}. Should be larger than \
                                zero and less than the population size.",
                               self.participants));
        }

        let mut result: Parents<T> = Vec::new();
        let mut rng = ::rand::thread_rng();
        for _ in 0..(self.count / 2) {
            let mut tournament: Vec<Box<T>> = Vec::with_capacity(self.participants);
            for _ in 0..self.participants {
                let index = rng.gen_range::<usize>(0, population.len());
                tournament.push(population[index].clone());
            }
            tournament.sort_by(|x, y| {
                (*x).fitness().partial_cmp(&(*y).fitness()).unwrap_or(Ordering::Equal)
            });
            match fitness_type {
                FitnessType::Maximize => {
                    result.push((tournament[tournament.len() - 1].clone(),
                                 tournament[tournament.len() - 2].clone()));
                }
                FitnessType::Minimize => {
                    result.push((tournament[0].clone(), tournament[1].clone()));
                }
            }
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use ::sim::*;
    use ::sim::select::*;
    use ::pheno::*;
    use std::cmp;

    #[derive(Clone)]
    struct Test {
        f: i64,
    }

    impl Phenotype for Test {
        fn fitness(&self) -> f64 {
            (self.f - 0).abs() as f64
        }

        fn crossover(&self, t: &Test) -> Test {
            Test { f: cmp::min(self.f, t.f) }
        }

        fn mutate(&self) -> Test {
            if self.f < 0 {
                Test { f: self.f + 1 }
            } else if self.f > 0 {
                Test { f: self.f - 1 }
            } else {
                self.clone()
            }
        }
    }

    #[test]
    fn test_count_zero() {
        let selector = TournamentSelector::new(0, 1);
        let population: Vec<Box<Test>> = (0..100).map(|i| Box::new(Test { f: i })).collect();
        assert!(selector.select(&population, FitnessType::Minimize).is_err());
    }

    #[test]
    fn test_participants_zero() {
        let selector = TournamentSelector::new(2, 0);
        let population: Vec<Box<Test>> = (0..100).map(|i| Box::new(Test { f: i })).collect();
        assert!(selector.select(&population, FitnessType::Minimize).is_err());
    }

    #[test]
    fn test_count_odd() {
        let selector = TournamentSelector::new(5, 1);
        let population: Vec<Box<Test>> = (0..100).map(|i| Box::new(Test { f: i })).collect();
        assert!(selector.select(&population, FitnessType::Minimize).is_err());
    }

    #[test]
    fn test_count_too_large() {
        let selector = TournamentSelector::new(100, 1);
        let population: Vec<Box<Test>> = (0..100).map(|i| Box::new(Test { f: i })).collect();
        assert!(selector.select(&population, FitnessType::Minimize).is_err());
    }

    #[test]
    fn test_participants_too_large() {
        let selector = TournamentSelector::new(2, 100);
        let population: Vec<Box<Test>> = (0..100).map(|i| Box::new(Test { f: i })).collect();
        assert!(selector.select(&population, FitnessType::Minimize).is_err());
    }

    #[test]
    fn test_result_size() {
        let selector = TournamentSelector::new(20, 5);
        let population: Vec<Box<Test>> = (0..100).map(|i| Box::new(Test { f: i })).collect();
        assert_eq!(20,
                   selector.select(&population, FitnessType::Minimize).unwrap().len() * 2);
    }
}

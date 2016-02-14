//! Contains a sequential implementation of `::sim::Simulation`,
//! called a `Simulator`.
//!
//! To use a `Simulator`, you need a `SimulatorBuilder`, which you can
//! obtain by calling `Simulator::builder()`.

use pheno::Phenotype;
use std::cmp::Ordering;
use rand::Rng;
use super::*;
use super::select::*;
use super::iterlimit::*;
use super::earlystopper::*;
use time::SteadyTime;

/// A sequential implementation of `::sim::Simulation`.
/// The genetic algorithm is run in a single thread.
pub struct Simulator<T: Phenotype>
{
    population: Vec<Box<T>>,
    iter_limit: IterLimit,
    selector: Box<Selector<T>>,
    fitness_type: FitnessType,
    earlystopper: Option<EarlyStopper>,
    duration: Option<NanoSecond>,
    error: Option<String>,
}

impl <T: Phenotype> Clone for Simulator<T> {
    fn clone(&self) -> Self {
        Simulator {
            population: self.population.clone(),
            iter_limit: self.iter_limit.clone(),
            fitness_type: self.fitness_type.clone(),
            earlystopper: self.earlystopper.clone(),
            duration: self.duration.clone(),
            error: self.error.clone(),
            selector: self.selector, // TODO: https://users.rust-lang.org/t/solved-is-it-possible-to-clone-a-boxed-trait-object/1714/5
        }
    }
}

impl<T: Phenotype> Simulation<T> for Simulator<T> {
    type B = SimulatorBuilder<T>;

    /// Create builder.
    fn builder() -> SimulatorBuilder<T> {
        SimulatorBuilder {
            sim: Simulator {
                population: Vec::new(),
                iter_limit: IterLimit::new(100),
                selector: Box::new(MaximizeSelector::new(4)),
                fitness_type: FitnessType::Maximize,
                earlystopper: None,
                duration: Some(0),
                error: None,
            },
        }
    }

    fn step(&mut self) -> StepResult {
        if self.population.is_empty() {
            self.error = Some(format!("Tried to run a simulator without a population, \
                                       or the population was empty."));
            return StepResult::Failure;
        }
        let time_start = SteadyTime::now();
        let should_stop = match self.earlystopper {
            Some(ref x) => self.iter_limit.reached() || x.reached(),
            None => self.iter_limit.reached(),
        };
        if should_stop {
            return StepResult::Done;
        } else {
            // Perform selection
            let parents_tmp = (*self.selector).select(&self.population, self.fitness_type);
            if parents_tmp.is_err() {
                self.error = Some(parents_tmp.err().unwrap());
                return StepResult::Failure;
            }
            let parents = parents_tmp.ok().unwrap();
            // Create children from the selected parents and mutate them.
            let mut children: Vec<Box<T>> = parents.iter()
                                                   .map(|pair: &(Box<T>, Box<T>)| {
                                                       pair.0.crossover(&*(pair.1))
                                                   })
                                                   .map(|c| Box::new(c.mutate()))
                                                   .collect();
            // Kill off parts of the population at random to make room for the children
            self.kill_off(children.len());
            self.population.append(&mut children);

            if let Some(ref mut stopper) = self.earlystopper {
                let mut cloned = self.population.clone();
                cloned.sort_by(|x, y| {
                    (*x).fitness().partial_cmp(&(*y).fitness()).unwrap_or(Ordering::Equal)
                });
                let highest_fitness = match self.fitness_type {
                                          FitnessType::Maximize => cloned[cloned.len() - 1].clone(),
                                          FitnessType::Minimize => cloned[0].clone(),
                                      }
                                      .fitness();
                stopper.update(highest_fitness);
            }

            self.iter_limit.inc();
        }
        let this_time = (SteadyTime::now() - time_start).num_nanoseconds();
        self.duration = match self.duration {
            Some(x) => {
                match this_time {
                    Some(y) => Some(x + y),
                    None => None,
                }
            }
            None => None,
        };
        StepResult::Success // Not done yet, but successful
    }

    /// Run.
    fn run(&mut self) -> RunResult {
        // Loop until Failure or Done.
        loop {
            match self.step() {
                StepResult::Success => {}
                StepResult::Failure => return RunResult::Failure,
                StepResult::Done => return RunResult::Done,
            }
        }
    }

    fn get(&self) -> SimResult<T> {
        match self.error {
            Some(ref e) => Err(e.clone()),
            None => {
                let mut cloned = self.population.clone();
                cloned.sort_by(|x, y| {
                    (*x).fitness().partial_cmp(&(*y).fitness()).unwrap_or(Ordering::Equal)
                });
                Ok(match self.fitness_type {
                    FitnessType::Maximize => cloned[cloned.len() - 1].clone(),
                    FitnessType::Minimize => cloned[0].clone(),
                })
            }
        }
    }

    fn population(&self) -> Option<&Vec<Box<T>>> {
        Some(&self.population)
    }

    fn iterations(&self) -> u64 {
        self.iter_limit.get()
    }

    fn time(&self) -> Option<NanoSecond> {
        self.duration
    }
}

impl<T: Phenotype> Simulator<T> {
    /// Kill off phenotypes using stochastic universal sampling.
    fn kill_off(&mut self, count: usize) {
        let ratio = self.population.len() / count;
        let mut i = ::rand::thread_rng().gen_range::<usize>(0, self.population.len());
        let mut selected = 0;
        while selected < count {
            self.population.remove(i);
            i += ratio - 1;
            i = i % self.population.len();

            selected += 1;
        }
    }
}

/// A `Builder` for the `Simulator` type.
pub struct SimulatorBuilder<T: Phenotype>
{
    sim: Simulator<T>,
}

impl<T: Phenotype> SimulatorBuilder<T> {
    /// Set the population of the resulting `Simulator`.
    ///
    /// Returns itself for chaining purposes.
    pub fn set_population(mut self, pop: &Vec<Box<T>>) -> Self {
        self.sim.population = pop.clone();
        self
    }

    /// Set the maximum number of iterations of the resulting `Simulator`.
    ///
    /// The `Simulator` will stop running after this number of iterations.
    ///
    /// Returns itself for chaining purposes.
    pub fn set_max_iters(mut self, i: u64) -> Self {
        self.sim.iter_limit = IterLimit::new(i);
        self
    }

    /// Set the fitness type of the resulting `Simulator`,
    /// determining whether the `Simulator` will try to maximize
    /// or minimize the fitness values of `Phenotype`s.
    ///
    /// Returns itself for chaining purposes.
    pub fn set_fitness_type(mut self, t: FitnessType) -> Self {
        self.sim.fitness_type = t;
        self
    }

    /// Set early stopping. If for `n_iters` iterations, the change in the highest fitness
    /// is smaller than `delta`, the simulator will stop running.
    ///
    /// Returns itself for chaining purposes.
    pub fn set_early_stop(mut self, delta: f64, n_iters: u64) -> Self {
        self.sim.earlystopper = Some(EarlyStopper::new(delta, n_iters));
        self
    }

    /// Set the selector.
    ///
    /// Returns itself for chaining purposes.
    pub fn set_selector(mut self, selector: Box<Selector<T>>) -> Self {
        self.sim.selector = selector;
        self
    }
}

impl<T: Phenotype> Builder<Box<Simulator<T>>> for SimulatorBuilder<T> {
    fn build(self) -> Box<Simulator<T>> {
        Box::new(self.sim)
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
    fn test_kill_off_count() {
        let selector = MaximizeSelector::new(2);
        let population: Vec<Box<Test>> = (0..100).map(|i| Box::new(Test { f: i })).collect();
        let mut s = *seq::Simulator::builder()
                         .set_population(&population)
                         .set_selector(Box::new(selector))
                         .build();
        s.kill_off(10);
        assert_eq!(s.population.len(), 90);
    }

    #[test]
    fn test_max_iters() {
        let selector = MaximizeSelector::new(2);
        let population: Vec<Box<Test>> = (0..100).map(|i| Box::new(Test { f: i })).collect();
        let mut s = *seq::Simulator::builder()
                         .set_population(&population)
                         .set_selector(Box::new(selector))
                         .set_max_iters(2)
                         .build();
        s.run();
        assert!(s.iterations() <= 2);
    }

    #[test]
    fn test_early_stopping() {
        let selector = MaximizeSelector::new(2);
        let population: Vec<Box<Test>> = (0..100).map(|_| Box::new(Test { f: 0 })).collect();
        let mut s = *seq::Simulator::builder()
                         .set_population(&population)
                         .set_early_stop(10.0, 5)
                         .set_selector(Box::new(selector))
                         .set_max_iters(10)
                         .build();
        s.run();
        assert!(s.iterations() <= 5);
    }

    #[test]
    fn test_selector_error_propagate() {
        let selector = MaximizeSelector::new(0);
        let population: Vec<Box<Test>> = (0..100).map(|i| Box::new(Test { f: i })).collect();
        let mut s = *seq::Simulator::builder()
                         .set_population(&population)
                         .set_selector(Box::new(selector))
                         .build();
        s.run();
        assert!(s.get().is_err());
    }

    #[test]
    fn test_no_population() {
        let mut s: seq::Simulator<Test> = *seq::Simulator::builder().build();
        s.run();
        assert!(s.get().is_err());
    }
}

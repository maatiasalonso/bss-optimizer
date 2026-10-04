use crate::domain::{DistrictingSolution, StationIndex};

/// Port for mathematical optimization solvers.
/// Solves the assignment of stations to the given fixed hubs.
pub trait AssignmentSolverPort: Send + Sync {
    fn solve(
        &self,
        hubs: &[StationIndex],
        balance_ratio: f64,
        priority_tolerance: i32,
        all_priorities: bool,
    ) -> Option<DistrictingSolution>;
}

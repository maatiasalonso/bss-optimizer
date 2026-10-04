use crate::application::ports::AssignmentSolverPort;
use crate::domain::{
    DistanceMatrix, DistrictingSolution, Meters, PriorityBuckets, ReachabilityMap, Station,
    StationIndex,
};
use good_lp::{
    constraint, default_solver, variable, variables, Expression, ResolutionError, Solution,
    SolverModel, Variable,
};
use good_lp::WithTimeLimit;

pub struct HighsAssignmentSolver<'a> {
    stations: &'a [Station],
    distances: &'a DistanceMatrix,
    reachability: &'a ReachabilityMap,
    priority_buckets: &'a PriorityBuckets,
    dmax: Meters,
    time_limit_seconds: Option<f64>,
}

impl<'a> std::fmt::Debug for HighsAssignmentSolver<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HighsAssignmentSolver")
            .field("num_stations", &self.stations.len())
            .field("dmax", &self.dmax)
            .field("time_limit_seconds", &self.time_limit_seconds)
            .finish()
    }
}

impl<'a> HighsAssignmentSolver<'a> {
    pub fn new(
        stations: &'a [Station],
        distances: &'a DistanceMatrix,
        reachability: &'a ReachabilityMap,
        priority_buckets: &'a PriorityBuckets,
        dmax: Meters,
    ) -> Self {
        Self {
            stations,
            distances,
            reachability,
            priority_buckets,
            dmax,
            time_limit_seconds: None,
        }
    }

    pub fn with_time_limit(mut self, seconds: Option<f64>) -> Self {
        self.time_limit_seconds = seconds;
        self
    }
}

impl<'a> AssignmentSolverPort for HighsAssignmentSolver<'a> {
    fn solve(
        &self,
        hubs: &[StationIndex],
        balance_ratio: f64,
        priority_tolerance: i32,
        all_priorities: bool,
    ) -> Option<DistrictingSolution> {
        let n = self.stations.len();
        let k = hubs.len();

        // 1. Fast O(N) pre-check: verify every station can reach at least one hub
        if !self.reachability.is_coverable(self.distances, hubs, self.dmax, n) {
            return None;
        }

        // 2. Build sparse variables
        // x_vars maps (station_idx, hub_idx) -> Variable
        let mut vars = variables!();
        let mut x_by_station: Vec<Vec<(usize, Variable)>> = vec![Vec::new(); n];
        let mut x_by_hub: Vec<Vec<(usize, Variable)>> = vec![Vec::new(); k];
        let mut x_by_hub_priority: Vec<[Vec<Variable>; 4]> = (0..k)
            .map(|_| [Vec::new(), Vec::new(), Vec::new(), Vec::new()])
            .collect();
        let mut obj_expr = Expression::from(0.0);

        for (h_idx, &hub_st) in hubs.iter().enumerate() {
            // Using sparse reachability index: only visit stations reachable from this hub
            let reachable = self.reachability.reachable_from(hub_st);
            for &st_idx in reachable {
                let dist = self.distances.dist(st_idx, hub_st).0;
                let var = vars.add(variable().binary());
                obj_expr += var * (dist as f64);
                x_by_station[st_idx.0].push((h_idx, var));
                x_by_hub[h_idx].push((st_idx.0, var));
                x_by_hub_priority[h_idx][self.stations[st_idx.0].priority.as_index()].push(var);
            }
        }

        let mut model = vars.minimise(obj_expr).using(default_solver);
        if let Some(seconds) = self.time_limit_seconds {
            model = model.with_time_limit(seconds);
        }

        // Constraint 1: Exact station assignment (sum_j x_ij == 1)
        for station_vars in &x_by_station {
            let mut assign_expr = Expression::from(0.0);
            for &(_, var) in station_vars {
                assign_expr += var;
            }
            model.add_constraint(constraint!(assign_expr == 1.0));
        }

        // Constraint 2: Priority balance (using Sparse Priority Buckets - NO 3-LEVEL NESTED LOOP!)
        let cl = k as f64;
        let priority_levels = if all_priorities { 0..4 } else { 0..1 };
        let tol = priority_tolerance as f64;

        for l in priority_levels {
            let bucket = self.priority_buckets.get_stations(l);
            let psum = bucket.len() as f64;
            let target = (psum / cl).floor();

            for hub_priority in x_by_hub_priority.iter().take(k) {
                let mut prior_expr = Expression::from(0.0);
                for &var in &hub_priority[l] {
                    prior_expr += var;
                }
                model.add_constraint(constraint!(prior_expr.clone() - target <= tol));
                model.add_constraint(constraint!(prior_expr - target >= -tol));
            }
        }

        // Constraint 3: Net flow balance (using Sparse Hub Reachability)
        let b = balance_ratio;
        for hub_vars in &x_by_hub {
            let mut flow_c1 = Expression::from(0.0);
            let mut flow_c2 = Expression::from(0.0);

            // Iterate only over stations reachable from this hub, not all N stations!
            for &(st_idx, var) in hub_vars {
                let st = &self.stations[st_idx];
                let r_minus = st.r_minus as f64;
                let r_plus = st.r_plus as f64;

                let coeff1 = (1.0 - b) * r_minus - (1.0 + b) * r_plus;
                let coeff2 = -(1.0 + b) * r_minus + (1.0 - b) * r_plus;

                if coeff1.abs() > 1e-6 {
                    flow_c1 += var * coeff1;
                }
                if coeff2.abs() > 1e-6 {
                    flow_c2 += var * coeff2;
                }
            }

            model.add_constraint(constraint!(flow_c1 <= 0.0));
            model.add_constraint(constraint!(flow_c2 <= 0.0));
        }

        // 3. Solve with HiGHS
        match model.solve() {
            Ok(solution) => {
                let mut assignments = vec![hubs[0]; n];
                for st_idx in 0..n {
                    let mut best_h = hubs[0];
                    let mut max_val = 0.0;
                    for &(h_idx, var) in &x_by_station[st_idx] {
                        let val = solution.value(var);
                        if val > max_val {
                            max_val = val;
                            best_h = hubs[h_idx];
                        }
                    }
                    assignments[st_idx] = best_h;
                }

                Some(DistrictingSolution::build(
                    hubs,
                    &assignments,
                    self.stations,
                    self.distances,
                ))
            }
            Err(ResolutionError::Infeasible) | Err(ResolutionError::Unbounded) => None,
            Err(e) => {
                eprintln!("HiGHS solver error: {:?}", e);
                None
            }
        }
    }
}

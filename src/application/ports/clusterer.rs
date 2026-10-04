use crate::domain::{Station, StationIndex};

/// Port for spatial clustering algorithms.
pub trait SpatialClustererPort: Send + Sync {
    fn algorithm_name(&self) -> &'static str;

    fn partition(
        &self,
        stations: &[Station],
        num_clusters: usize,
        seed: u64,
    ) -> Vec<Vec<StationIndex>>;
}

use super::station::Station;
use super::types::{Meters, StationIndex};

/// Encapsulates the flat distance matrix between all stations in the network.
#[derive(Debug, Clone)]
pub struct DistanceMatrix {
    num_stations: usize,
    distances: Vec<u32>,
}

impl DistanceMatrix {
    pub fn new(num_stations: usize, distances: Vec<u32>) -> Self {
        assert_eq!(
            distances.len(),
            num_stations * num_stations,
            "Distance matrix dimension mismatch"
        );
        Self {
            num_stations,
            distances,
        }
    }

    #[inline(always)]
    pub fn dist(&self, i: StationIndex, j: StationIndex) -> Meters {
        Meters(self.distances[i.0 * self.num_stations + j.0])
    }
}

/// Sparse Inverted Reachability Index.
/// For each station, pre-indexes only the stations within the maximum driving cutoff dmax.
/// This completely eliminates the need for dense nested loops during MIP model generation.
#[derive(Debug, Clone)]
pub struct ReachabilityMap {
    /// stations_within_dmax[i] = list of j where dist(i, j) <= dmax
    neighbors_within_dmax: Vec<Vec<StationIndex>>,
}

impl ReachabilityMap {
    pub fn build(matrix: &DistanceMatrix, num_stations: usize, dmax: Meters) -> Self {
        let mut neighbors = Vec::with_capacity(num_stations);
        for i in 0..num_stations {
            let mut reachable = Vec::new();
            for j in 0..num_stations {
                if matrix.dist(StationIndex(i), StationIndex(j)) <= dmax {
                    reachable.push(StationIndex(j));
                }
            }
            neighbors.push(reachable);
        }
        Self {
            neighbors_within_dmax: neighbors,
        }
    }

    /// Fast lookup: returns the list of stations reachable from center within dmax
    #[inline(always)]
    pub fn reachable_from(&self, center: StationIndex) -> &[StationIndex] {
        &self.neighbors_within_dmax[center.0]
    }

    /// O(N) fast coverability check: verifies that every station can reach at least one of the selected hubs
    pub fn is_coverable(
        &self,
        _matrix: &DistanceMatrix,
        hubs: &[StationIndex],
        _dmax: Meters,
        num_stations: usize,
    ) -> bool {
        let mut covered = vec![false; num_stations];
        for &hub in hubs {
            for &station in self.reachable_from(hub) {
                covered[station.0] = true;
            }
        }
        if covered.iter().any(|&is_covered| !is_covered) {
                return false;
        }
        true
    }
}

/// Pre-grouped station indices bucketed by operational priority tier.
/// Allows O(1) retrieval of all Tier-1 stations rather than scanning the entire city.
#[derive(Debug, Clone)]
pub struct PriorityBuckets {
    pub tier_stations: [Vec<StationIndex>; 4],
}

impl PriorityBuckets {
    pub fn from_stations(stations: &[Station]) -> Self {
        let mut buckets = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
        for st in stations {
            let idx = st.priority.as_index();
            buckets[idx].push(st.index);
        }
        Self {
            tier_stations: buckets,
        }
    }

    #[inline(always)]
    pub fn get_stations(&self, tier_idx: usize) -> &[StationIndex] {
        &self.tier_stations[tier_idx]
    }
}

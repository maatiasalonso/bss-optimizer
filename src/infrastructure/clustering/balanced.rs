use super::kmeans::NaiveKMeansClusterer;
use crate::application::ports::SpatialClustererPort;
use crate::domain::{Station, StationIndex};

#[derive(Debug, Clone, Copy, Default)]
pub struct BalancedKMeansClusterer;

impl SpatialClustererPort for BalancedKMeansClusterer {
    fn algorithm_name(&self) -> &'static str {
        "Balanced / Constrained K-Means"
    }

    fn partition(
        &self,
        stations: &[Station],
        num_clusters: usize,
        seed: u64,
    ) -> Vec<Vec<StationIndex>> {
        let naive = NaiveKMeansClusterer;
        let mut partitions = naive.partition(stations, num_clusters, seed);

        let target_size = stations.len() / num_clusters;
        let max_allowed = target_size + 4;

        // Rebalance sizes greedily by geographic centroid proximity
        for _ in 0..10 {
            let mut transferred = false;
            for c_from in 0..num_clusters {
                if partitions[c_from].len() > max_allowed {
                    let candidates = partitions[c_from].clone();
                    for st_idx in candidates {
                        let mut best_target = None;
                        let mut best_dist = f64::INFINITY;
                        let s = &stations[st_idx.0];

                        for (c_to, target_members) in partitions.iter().enumerate() {
                            if c_to != c_from && target_members.len() < max_allowed && !target_members.is_empty() {
                                let avg_d: f64 = target_members
                                        .iter()
                                        .map(|&m| {
                                            let other = &stations[m.0];
                                            (s.lat - other.lat).powi(2) + (s.lon - other.lon).powi(2)
                                        })
                                        .sum::<f64>()
                                        / target_members.len() as f64;
                                    if avg_d < best_dist {
                                        best_dist = avg_d;
                                        best_target = Some(c_to);
                                    }
                            }
                        }

                        if let Some(target) = best_target {
                            partitions[c_from].retain(|&x| x != st_idx);
                            partitions[target].push(st_idx);
                            transferred = true;
                            if partitions[c_from].len() <= max_allowed {
                                break;
                            }
                        }
                    }
                }
            }
            if !transferred {
                break;
            }
        }

        partitions
    }
}

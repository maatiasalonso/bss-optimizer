use crate::application::ports::SpatialClustererPort;
use crate::domain::{Station, StationIndex};
use rand::prelude::*;
use rand::rngs::StdRng;

#[derive(Debug, Clone, Copy, Default)]
pub struct NaiveKMeansClusterer;

impl SpatialClustererPort for NaiveKMeansClusterer {
    fn algorithm_name(&self) -> &'static str {
        "Naive K-Means"
    }

    fn partition(
        &self,
        stations: &[Station],
        num_clusters: usize,
        seed: u64,
    ) -> Vec<Vec<StationIndex>> {
        let n = stations.len();
        let k = num_clusters;
        let mut rng = StdRng::seed_from_u64(seed);

        let mut centroids = Vec::with_capacity(k);
        let mut chosen = Vec::new();
        while centroids.len() < k {
            let idx = rng.gen_range(0..n);
            if !chosen.contains(&idx) {
                chosen.push(idx);
                centroids.push((stations[idx].lat, stations[idx].lon));
            }
        }

        let mut assignments = vec![0usize; n];
        for _ in 0..100 {
            let mut changed = false;
            for i in 0..n {
                let s = &stations[i];
                let mut best_dist = f64::INFINITY;
                let mut best_c = 0;
                for (c, &(c_lat, c_lon)) in centroids.iter().enumerate() {
                    let d2 = (s.lat - c_lat).powi(2) + (s.lon - c_lon).powi(2);
                    if d2 < best_dist {
                        best_dist = d2;
                        best_c = c;
                    }
                }
                if assignments[i] != best_c {
                    assignments[i] = best_c;
                    changed = true;
                }
            }

            if !changed {
                break;
            }

            let mut counts = vec![0usize; k];
            let mut sum_lat = vec![0.0f64; k];
            let mut sum_lon = vec![0.0f64; k];

            for i in 0..n {
                let c = assignments[i];
                counts[c] += 1;
                sum_lat[c] += stations[i].lat;
                sum_lon[c] += stations[i].lon;
            }

            for c in 0..k {
                if counts[c] > 0 {
                    centroids[c] = (sum_lat[c] / counts[c] as f64, sum_lon[c] / counts[c] as f64);
                } else {
                    let idx = rng.gen_range(0..n);
                    centroids[c] = (stations[idx].lat, stations[idx].lon);
                }
            }
        }

        let mut partitions: Vec<Vec<StationIndex>> = vec![Vec::new(); k];
        for (i, &c) in assignments.iter().enumerate() {
            partitions[c].push(StationIndex(i));
        }

        // Fill any empty clusters
        for c in 0..k {
            if partitions[c].is_empty() {
                let mut max_c = 0;
                let mut max_len = 0;
                for (other, p) in partitions.iter().enumerate() {
                    if p.len() > max_len {
                        max_len = p.len();
                        max_c = other;
                    }
                }
                if max_len > 1 {
                    let st = partitions[max_c].pop().unwrap();
                    partitions[c].push(st);
                }
            }
        }

        partitions
    }
}

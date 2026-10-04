use super::station::Station;
use super::types::{Meters, NetFlow, StationId, StationIndex};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct District {
    pub cluster_idx: usize,
    pub hub_station: StationIndex,
    pub hub_id: StationId,
    pub member_stations: Vec<StationIndex>,
    pub net_flow: NetFlow,
    pub total_flow: i32,
    pub max_radius: Meters,
    pub avg_distance: f64,
    pub priority_distribution: [usize; 4],
}

impl District {
    pub fn station_count(&self) -> usize {
        self.member_stations.len()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistrictingSolution {
    pub total_distance: Meters,
    pub hubs: Vec<StationIndex>,
    pub assignments: Vec<StationIndex>,
    pub districts: Vec<District>,
    pub avg_radius: f64,
    pub max_radius: Meters,
}

impl DistrictingSolution {
    pub fn build(
        hubs: &[StationIndex],
        assignments: &[StationIndex],
        stations: &[Station],
        distances: &super::spatial::DistanceMatrix,
    ) -> Self {
        let num_hubs = hubs.len();
        let mut member_lists: Vec<Vec<StationIndex>> = vec![Vec::new(); num_hubs];
        let mut hub_positions = vec![None; stations.len()];
        for (position, &hub) in hubs.iter().enumerate() {
            hub_positions[hub.0] = Some(position);
        }
        let mut total_distance = 0u64;

        for (st_idx, &hub_st) in assignments.iter().enumerate() {
            let hub_pos = hub_positions[hub_st.0].unwrap_or(0);
            member_lists[hub_pos].push(StationIndex(st_idx));
            total_distance += distances.dist(StationIndex(st_idx), hub_st).0 as u64;
        }

        let mut districts = Vec::with_capacity(num_hubs);
        let mut max_radii = Vec::with_capacity(num_hubs);

        for (h_pos, &hub_st) in hubs.iter().enumerate() {
            let members = &member_lists[h_pos];
            let mut max_r = 0u32;
            let mut sum_dist = 0.0;
            let mut net = 0i32;
            let mut total = 0i32;
            let mut p_counts = [0usize; 4];

            for &m in members {
                let d = distances.dist(m, hub_st).0;
                if d > max_r {
                    max_r = d;
                }
                sum_dist += d as f64;
                let s = &stations[m.0];
                net += s.r_minus - s.r_plus;
                total += s.r_minus + s.r_plus;
                p_counts[s.priority.as_index()] += 1;
            }

            let avg_d = if !members.is_empty() {
                sum_dist / members.len() as f64
            } else {
                0.0
            };

            max_radii.push(max_r);
            districts.push(District {
                cluster_idx: h_pos,
                hub_station: hub_st,
                hub_id: stations[hub_st.0].id,
                member_stations: members.clone(),
                net_flow: NetFlow(net),
                total_flow: total,
                max_radius: Meters(max_r),
                avg_distance: avg_d,
                priority_distribution: p_counts,
            });
        }

        let avg_radius = if !max_radii.is_empty() {
            max_radii.iter().map(|&r| r as f64).sum::<f64>() / max_radii.len() as f64
        } else {
            0.0
        };
        let max_radius = Meters(*max_radii.iter().max().unwrap_or(&0));

        Self {
            total_distance: Meters(total_distance as u32),
            hubs: hubs.to_vec(),
            assignments: assignments.to_vec(),
            districts,
            avg_radius,
            max_radius,
        }
    }
}

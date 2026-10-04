use crate::application::ports::SpatialClustererPort;
use crate::domain::{Station, StationIndex};

#[derive(Debug, Clone, Copy, Default)]
pub struct ManualGridClusterer;

impl SpatialClustererPort for ManualGridClusterer {
    fn algorithm_name(&self) -> &'static str {
        "Manual Geographic Grid (Legacy)"
    }

    fn partition(
        &self,
        stations: &[Station],
        num_clusters: usize,
        _seed: u64,
    ) -> Vec<Vec<StationIndex>> {
        if num_clusters != 15 {
            return parameterized_grid(stations, num_clusters);
        }

        let mut partitions: Vec<Vec<StationIndex>> = vec![Vec::new(); num_clusters];

        let min_lat = 19.3582;
        let max_lat = 19.444033;
        let med_lat = (max_lat + min_lat) / 2.0;
        let quarter_lat = 19.42;
        let a = min_lat + (med_lat - min_lat) / 3.0;
        let b = a + (med_lat - min_lat) / 3.0;

        let min_long = -99.20781;
        let max_long = -99.13;
        let amp1 = (max_long - min_long) / 6.0;
        let l1 = min_long + amp1;
        let l2 = l1 + amp1;
        let l3 = l2 + amp1;
        let l4 = l3 + amp1;
        let l5 = l4 + amp1;

        let c = -99.1910;
        let d = -99.15;
        let amp2 = (d - c) / 3.0;
        let l6 = c + amp2;
        let l7 = l6 + amp2;
        let amp3 = (d - c) / 2.0;
        let l8 = c + amp3;

        let boxes: [(f64, f64, f64, f64); 15] = [
            (min_long, l1, quarter_lat, max_lat),
            (l1, l2, quarter_lat, max_lat),
            (l2, l3, quarter_lat, max_lat),
            (l3, l4, quarter_lat, max_lat),
            (l4, l5, quarter_lat, max_lat),
            (l5, max_long, quarter_lat, max_lat),
            (c, l6, med_lat, quarter_lat),
            (l6, l7, med_lat, quarter_lat),
            (l7, d, med_lat, quarter_lat),
            (c, l8, b, med_lat),
            (l8, d, b, med_lat),
            (c, l8, a, b),
            (l8, d, a, b),
            (c, l8, min_lat, a),
            (l8, d, min_lat, a),
        ];

        for st in stations {
            let mut matched = false;
            for (z, &(x1, x2, y1, y2)) in boxes.iter().enumerate().take(num_clusters) {
                let (min_x, max_x) = if x1 < x2 { (x1, x2) } else { (x2, x1) };
                let (min_y, max_y) = if y1 < y2 { (y1, y2) } else { (y2, y1) };
                if st.lon >= min_x && st.lon <= max_x && st.lat >= min_y && st.lat <= max_y {
                    partitions[z].push(st.index);
                    matched = true;
                    break;
                }
            }
            if !matched {
                let mut closest_z = 0;
                let mut min_d = f64::INFINITY;
                for (z, &(x1, x2, y1, y2)) in boxes.iter().enumerate().take(num_clusters) {
                    let center_x = (x1 + x2) / 2.0;
                    let center_y = (y1 + y2) / 2.0;
                    let d2 = (st.lon - center_x).powi(2) + (st.lat - center_y).powi(2);
                    if d2 < min_d {
                        min_d = d2;
                        closest_z = z;
                    }
                }
                partitions[closest_z].push(st.index);
            }
        }

        // Fill any empty clusters
        for c in 0..num_clusters {
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

fn parameterized_grid(stations: &[Station], num_clusters: usize) -> Vec<Vec<StationIndex>> {
    assert!(num_clusters > 0, "the number of clusters must be positive");
    assert!(
        num_clusters <= stations.len(),
        "the number of clusters cannot exceed the number of stations"
    );

    let min_lat = stations
        .iter()
        .map(|station| station.lat)
        .fold(f64::INFINITY, f64::min);
    let max_lat = stations
        .iter()
        .map(|station| station.lat)
        .fold(f64::NEG_INFINITY, f64::max);
    let min_lon = stations
        .iter()
        .map(|station| station.lon)
        .fold(f64::INFINITY, f64::min);
    let max_lon = stations
        .iter()
        .map(|station| station.lon)
        .fold(f64::NEG_INFINITY, f64::max);

    let columns = (num_clusters as f64).sqrt().ceil() as usize;
    let rows = num_clusters.div_ceil(columns);
    let lat_span = (max_lat - min_lat).max(f64::EPSILON);
    let lon_span = (max_lon - min_lon).max(f64::EPSILON);
    let mut partitions = vec![Vec::new(); num_clusters];

    for station in stations {
        let column = (((station.lon - min_lon) / lon_span) * columns as f64)
            .floor()
            .min((columns - 1) as f64) as usize;
        let row = (((station.lat - min_lat) / lat_span) * rows as f64)
            .floor()
            .min((rows - 1) as f64) as usize;
        let cell = row * columns + column;
        let cluster = cell.min(num_clusters - 1);
        partitions[cluster].push(station.index);
    }

    for empty in 0..num_clusters {
        if partitions[empty].is_empty() {
            let donor = (0..num_clusters)
                .filter(|&candidate| partitions[candidate].len() > 1)
                .max_by_key(|&candidate| partitions[candidate].len())
                .expect("enough stations for every cluster");
            let station = partitions[donor].pop().expect("donor cluster is nonempty");
            partitions[empty].push(station);
        }
    }

    partitions
}

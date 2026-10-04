use crate::domain::{
    DistanceMatrix, Meters, PriorityBuckets, PriorityTier, ReachabilityMap, Station, StationId,
    StationIndex,
};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct ParsedInstance {
    pub name: String,
    pub num_stations: usize,
    pub num_clusters: usize,
    pub stations: Vec<Station>,
    pub distances: DistanceMatrix,
    pub reachability: ReachabilityMap,
    pub priority_buckets: PriorityBuckets,
    pub dmax: Meters,
    pub default_balance: f64,
    pub default_priority: i32,
    pub default_allprior: bool,
}

pub fn load_instance_from_files<P1: AsRef<Path>, P2: AsRef<Path>>(
    dat_path: P1,
    geo_path: P2,
) -> Result<ParsedInstance, Box<dyn std::error::Error>> {
    let dat_content = fs::read_to_string(dat_path.as_ref())?;
    let geo_content = fs::read_to_string(geo_path.as_ref())?;

    let mut geo_map = HashMap::new();
    for line in geo_content.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 3 {
            let id: usize = parts[0].parse()?;
            let lat: f64 = parts[1].parse()?;
            let lon: f64 = parts[2].parse()?;
            geo_map.insert(id, (lat, lon));
        }
    }

    let raw_tokens: Vec<&str> = dat_content
        .split([':', '[', ']', '\n', '\r'])
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    let mut station_ids = Vec::new();
    let mut r_mas = Vec::new();
    let mut r_menos = Vec::new();
    let mut cl = 15usize;
    let mut dmax_raw = 2500u32;
    let mut balance = 0.2f64;
    let mut prioridad = 5i32;
    let mut allprior = false;
    let mut dist_flat = Vec::new();
    let mut prior_rows = Vec::new();

    let mut idx = 0;
    while idx < raw_tokens.len() {
        let key = raw_tokens[idx];
        match key {
            "ESTACIONES" => {
                idx += 1;
                station_ids = raw_tokens[idx]
                    .split_whitespace()
                    .map(|s| s.parse::<usize>().unwrap())
                    .collect();
            }
            "r_mas" => {
                idx += 1;
                r_mas = raw_tokens[idx]
                    .split_whitespace()
                    .map(|s| s.parse::<i32>().unwrap())
                    .collect();
            }
            "r_menos" => {
                idx += 1;
                r_menos = raw_tokens[idx]
                    .split_whitespace()
                    .map(|s| s.parse::<i32>().unwrap())
                    .collect();
            }
            "cl" => {
                idx += 1;
                cl = raw_tokens[idx].parse()?;
            }
            "dmax" => {
                idx += 1;
                dmax_raw = raw_tokens[idx].parse()?;
            }
            "balance" => {
                idx += 1;
                balance = raw_tokens[idx].parse()?;
            }
            "prioridad" => {
                idx += 1;
                prioridad = raw_tokens[idx].parse()?;
            }
            "restrc_allprior" => {
                idx += 1;
                allprior = raw_tokens[idx].parse::<i32>()? == 1;
            }
            "dist" => {
                idx += 1;
                let n = station_ids.len();
                dist_flat.reserve(n * n);
                for row_i in 0..n {
                    let line_tokens: Vec<u32> = raw_tokens[idx + row_i]
                        .split_whitespace()
                        .map(|s| s.parse::<u32>().unwrap())
                        .collect();
                    dist_flat.extend(line_tokens);
                }
                idx += n - 1;
            }
            "prior" => {
                idx += 1;
                let n = station_ids.len();
                for row_i in 0..n {
                    let line_tokens: Vec<i32> = raw_tokens[idx + row_i]
                        .split_whitespace()
                        .map(|s| s.parse::<i32>().unwrap())
                        .collect();
                    let mut vec4 = [0i32; 4];
                    for (k, v) in line_tokens.into_iter().take(4).enumerate() {
                        vec4[k] = v;
                    }
                    prior_rows.push(vec4);
                }
                idx += n - 1;
            }
            _ => {}
        }
        idx += 1;
    }

    let num_stations = station_ids.len();
    let mut stations = Vec::with_capacity(num_stations);

    for (i, &id) in station_ids.iter().enumerate() {
        let (lat, lon) = geo_map.get(&id).cloned().unwrap_or((0.0, 0.0));
        let p_vec = if i < prior_rows.len() {
            prior_rows[i]
        } else {
            [0, 0, 0, 0]
        };
        let p_tier_val = p_vec
            .iter()
            .position(|&x| x == 1)
            .map(|pos| (pos + 1) as u8)
            .unwrap_or(4);

        stations.push(Station {
            index: StationIndex(i),
            id: StationId(id),
            lat,
            lon,
            r_plus: r_mas.get(i).cloned().unwrap_or(0),
            r_minus: r_menos.get(i).cloned().unwrap_or(0),
            priority: PriorityTier::from_u8(p_tier_val),
            priority_vector: p_vec,
        });
    }

    let distances = DistanceMatrix::new(num_stations, dist_flat);
    let dmax = Meters(dmax_raw);
    let reachability = ReachabilityMap::build(&distances, num_stations, dmax);
    let priority_buckets = PriorityBuckets::from_stations(&stations);

    Ok(ParsedInstance {
        name: "PT_452_15".to_string(),
        num_stations,
        num_clusters: cl,
        stations,
        distances,
        reachability,
        priority_buckets,
        dmax,
        default_balance: balance,
        default_priority: prioridad,
        default_allprior: allprior,
    })
}

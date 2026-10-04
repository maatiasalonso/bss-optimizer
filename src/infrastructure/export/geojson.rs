use crate::domain::{DistanceMatrix, DistrictingSolution, Station};
use serde_json::{json, Value};
use std::collections::HashSet;
use std::fs;
use std::path::Path;

const DISTRICT_COLORS: [&str; 15] = [
    "#e6194B", "#3cb44b", "#ffe119", "#4363d8", "#f58231",
    "#911eb4", "#42d4f4", "#f032e6", "#bfef45", "#fabed4",
    "#469990", "#dcbeff", "#9A6324", "#fffac8", "#800000",
];

pub fn export_districts_geojson<P: AsRef<Path>>(
    stations: &[Station],
    solution: &DistrictingSolution,
    distances: &DistanceMatrix,
    path: P,
) -> std::io::Result<()> {
    let mut features = Vec::new();
    let hub_set: HashSet<usize> = solution.hubs.iter().map(|h| h.0).collect();

    for d in &solution.districts {
        let color = DISTRICT_COLORS[d.cluster_idx % DISTRICT_COLORS.len()];
        let hub_st = &stations[d.hub_station.0];

        for &st_idx in &d.member_stations {
            let st = &stations[st_idx.0];
            let is_hub = hub_set.contains(&st_idx.0);

            let pt = json!({
                "type": "Feature",
                "geometry": {
                    "type": "Point",
                    "coordinates": [st.lon, st.lat]
                },
                "properties": {
                    "station_id": st.id.0,
                    "district_idx": d.cluster_idx + 1,
                    "is_hub": is_hub,
                    "priority_tier": st.priority.as_index() + 1,
                    "bike_returns_r_plus": st.r_plus,
                    "bike_pickups_r_minus": st.r_minus,
                    "net_flow": st.net_flow().0,
                    "marker-color": color,
                    "marker-size": if is_hub { "large" } else { "medium" },
                    "marker-symbol": if is_hub { "star" } else { "bicycle" },
                    "title": format!("{} {}", st.id, if is_hub { "★ (HUB)" } else { "" })
                }
            });
            features.push(pt);

            if !is_hub {
                let dist = distances.dist(st.index, d.hub_station).0;
                let line = json!({
                    "type": "Feature",
                    "geometry": {
                        "type": "LineString",
                        "coordinates": [
                            [st.lon, st.lat],
                            [hub_st.lon, hub_st.lat]
                        ]
                    },
                    "properties": {
                        "district_idx": d.cluster_idx + 1,
                        "stroke": color,
                        "stroke-width": 1.5,
                        "stroke-opacity": 0.5,
                        "distance_meters": dist
                    }
                });
                features.push(line);
            }
        }
    }

    let geojson: Value = json!({
        "type": "FeatureCollection",
        "name": "BSS_Ecobici_Districts_SOTA",
        "features": features
    });

    if let Some(parent) = path.as_ref().parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_string_pretty(&geojson)?)
}

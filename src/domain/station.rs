use super::types::{NetFlow, PriorityTier, StationId, StationIndex};
use serde::{Deserialize, Serialize};

/// Station entity containing geographical location, operational priorities, and demand flows.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Station {
    pub index: StationIndex,
    pub id: StationId,
    pub lat: f64,
    pub lon: f64,
    pub r_plus: i32,  // Bike returns / dropoffs (r_mas)
    pub r_minus: i32, // Bike pickups (r_menos)
    pub priority: PriorityTier,
    pub priority_vector: [i32; 4],
}

impl Station {
    #[inline(always)]
    pub fn net_flow(&self) -> NetFlow {
        NetFlow(self.r_minus - self.r_plus)
    }

    #[inline(always)]
    pub fn total_activity(&self) -> i32 {
        self.r_minus + self.r_plus
    }
}

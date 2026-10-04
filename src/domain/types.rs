use serde::{Deserialize, Serialize};
use std::fmt;

/// Strongly-typed 0-based internal array index (0..num_stations-1)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct StationIndex(pub usize);

impl From<usize> for StationIndex {
    #[inline(always)]
    fn from(idx: usize) -> Self {
        Self(idx)
    }
}

impl fmt::Display for StationIndex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "idx:{}", self.0)
    }
}

/// Strongly-typed 1-based real-world ECOBICI Station ID (1..452)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct StationId(pub usize);

impl From<usize> for StationId {
    #[inline(always)]
    fn from(id: usize) -> Self {
        Self(id)
    }
}

impl fmt::Display for StationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Station #{}", self.0)
    }
}

/// Distance in meters
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Meters(pub u32);

impl fmt::Display for Meters {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} m", self.0)
    }
}

/// Operational Priority Tier for rebalancing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PriorityTier {
    Tier1 = 1, // Critical (Metros, Major Hubs)
    Tier2 = 2, // High
    Tier3 = 3, // Medium
    Tier4 = 4, // Low / Quiet Residential
}

impl PriorityTier {
    #[inline(always)]
    pub fn from_u8(val: u8) -> Self {
        match val {
            1 => Self::Tier1,
            2 => Self::Tier2,
            3 => Self::Tier3,
            _ => Self::Tier4,
        }
    }

    #[inline(always)]
    pub fn as_index(self) -> usize {
        (self as usize) - 1
    }
}

/// Net bike flow balance: (returns - pickups) or (r_minus - r_plus)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetFlow(pub i32);

impl fmt::Display for NetFlow {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0 > 0 {
            write!(f, "+{}", self.0)
        } else {
            write!(f, "{}", self.0)
        }
    }
}


#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "std")]
use std::convert::TryFrom;
use sp_runtime::RuntimeDebug;
use sp_core::{Encode, Decode};

/// Reported geodes' state
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub enum ReportType {
    /// Geode failed challange check
    Challenge = 0x00,
    /// Geode failed service check
    Service,
    /// Default type
    Default,
}

impl TryFrom<u8> for ReportType {
    type Error = ();

    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            x if x == ReportType::Challenge as u8 => Ok(ReportType::Challenge),
            x if x == ReportType::Service as u8 => Ok(ReportType::Service),
            x if x == ReportType::Default as u8 => Ok(ReportType::Default),
            _ => Err(()),
        }
    }
}

impl Default for ReportType {
    fn default() -> Self {
        ReportType::Default
    }
}
use sp_core::{Decode, Encode};

#[derive(Clone, Debug, Encode, Decode, PartialEq, Eq, PartialOrd, Ord, Copy)]
pub enum GeodeSessionPhase {
    SessionInitialize,
    GeodeOffline,
    OrderDispatch,
    ExpiredCheck,
}

impl GeodeSessionPhase {
    pub fn all() -> [GeodeSessionPhase; 4] {
        use GeodeSessionPhase::*;
        [SessionInitialize, GeodeOffline, OrderDispatch, ExpiredCheck]
    }

    pub fn next(&self) -> Self {
        let all = Self::all();
        for (idx, phase) in all.iter().enumerate() {
            if phase.eq(self) {
                return all[(idx + 1) % all.len()];
            }
        }
        unreachable!();
    }
}

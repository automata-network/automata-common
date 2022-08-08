#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_core::U256;
use sp_core::{Decode, Encode};
use sp_runtime::RuntimeDebug;
use sp_std::vec::Vec;

pub type OrderOf<T> = Order<
    <T as frame_system::Config>::Hash,
    <T as frame_system::Config>::BlockNumber,
    <T as frame_system::Config>::AccountId,
>;

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, Default)]
pub struct Order<Hash, BlockNumber, AccountId> {
    pub order_id: Hash,
    pub binary: Vec<u8>,
    pub domain: Vec<u8>,
    pub name: Vec<u8>,
    pub provider: AccountId,
    // token num that users are willing to pay
    pub price: U256,
    pub start_session_id: BlockNumber,
    // session num
    pub duration: BlockNumber,
    pub num: u32,
    pub state: OrderState,
    // price - refund_unit / duration * geode_num * price
    pub refund_unit: u32,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, Copy)]
pub enum OrderState {
    Submitted,
    Pending,
    Processing,
    Emergency,
    Done,
}

impl OrderState {
    pub fn check_next(&self, next: Self) -> bool {
        match self {
            Self::Submitted => next == Self::Pending,
            Self::Pending => [
                Self::Processing,
                Self::Emergency,
                Self::Done, // timeout
            ]
            .contains(&next),
            Self::Processing => [Self::Emergency, Self::Done].contains(&next),
            Self::Emergency => next == Self::Processing,
            Self::Done => false,
        }
    }
}

impl Default for OrderState {
    fn default() -> Self {
        Self::Submitted
    }
}

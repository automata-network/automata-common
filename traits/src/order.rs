use frame_support::pallet_prelude::DispatchResult;
use primitives::order::OrderState;

pub trait OrderTrait {
    type AccountId;
    type BlockNumber;
    type Hash;
    fn is_order_expired(order_id: Self::Hash, session_index: Self::BlockNumber) -> bool;
    fn on_new_session(session_index: Self::BlockNumber);
    fn on_orders_dispatch(session_index: Self::BlockNumber);
    fn on_emergency_order_dispatch(session_index: Self::BlockNumber);
    fn on_order_state(
        service_id: Self::AccountId,
        order_id: Self::Hash,
        target_state: OrderState,
    ) -> DispatchResult;
}

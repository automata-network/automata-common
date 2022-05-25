use frame_support::dispatch::DispatchResult;

pub trait GeodeTrait {
    type AccountId;
    type Hash;
    fn on_new_session(session_index: u32) -> DispatchResult;
    fn on_geode_offline(session_index: u32) -> DispatchResult;
    fn on_geode_unhealthy(geode_id: Self::AccountId) -> DispatchResult;
    fn on_order_dispatched(geode_id: Self::AccountId, order_id: Self::Hash) -> DispatchResult;
    fn on_expired_check();
}

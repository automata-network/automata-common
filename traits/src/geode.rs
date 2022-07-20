use frame_support::dispatch::DispatchResult;

pub trait GeodeTrait {
    type AccountId;
    type Hash;
    type BlockNumber;
    fn on_new_session(
        block_height: Self::BlockNumber,
        session_index: Self::BlockNumber,
    );
    fn on_geode_offline(session_index: Self::BlockNumber);
    fn on_geode_unhealthy(geode_id: Self::AccountId);
    fn on_order_dispatched(
        geode_id: Self::AccountId,
        session_index: Self::BlockNumber,
        order_id: Self::Hash,
    ) -> DispatchResult;
    fn on_expired_check(
        current_block_height: Self::BlockNumber,
        current_session_index: Self::BlockNumber,
    );
}

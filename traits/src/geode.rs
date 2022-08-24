use sp_std::vec::Vec;

pub trait GeodeTrait {
    type AccountId;
    type Hash;
    type BlockNumber;
    fn on_new_session(session_index: Self::BlockNumber);
    fn on_geode_offline(session_index: Self::BlockNumber);
    fn on_order_dispatched(
        session_index: Self::BlockNumber,
        order_id: Self::Hash,
        num: u32, // the number of geode for this order want to dispatch
        domain: Vec<u8>,
    ) -> Vec<Self::AccountId>;
    fn on_expired_check(session_index: Self::BlockNumber);
}

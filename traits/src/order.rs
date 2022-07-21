pub trait OrderTrait {
    type BlockNumber;
    type Hash;
    fn is_order_expired(order_id: Self::Hash, session_index: Self::BlockNumber) -> bool;
}

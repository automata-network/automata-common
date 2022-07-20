pub trait OrderTrait {
    type BlockNumber;
    type Hash;
    fn is_order_expired(
        order_id: Self::Hash,
        block_height: Self::BlockNumber,
        session_index: Self::BlockNumber,
    ) -> bool;
}

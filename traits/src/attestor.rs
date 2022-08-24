use frame_support::dispatch::DispatchResult;

pub trait ApplicationTrait {
    type AccountId;
    /// Currently we will only report a simple `unhealthy` state, but there might be more status in the future.
    /// E.g maybe something wrong with the application binary
    fn application_unhealthy(who: Self::AccountId, is_attestor_down: bool) -> DispatchResult;

    /// Application are attested by several attestors, and reach healthy state
    fn application_healthy(who: Self::AccountId) -> DispatchResult;
}

pub trait AttestorTrait {
    type AccountId;
    fn is_abnormal_mode() -> bool;
    fn check_healthy(app_id: &Self::AccountId) -> bool;
}

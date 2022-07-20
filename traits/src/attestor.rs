use frame_support::dispatch::DispatchResult;
use primitives::attestor::ReportType;

pub trait ApplicationTrait {
    type AccountId;
    /// Currently we will only report a simple `unhealthy` state, but there might be more status in the future.
    /// E.g maybe something wrong with the application binary
    fn application_unhealthy(who: Self::AccountId) -> DispatchResult;

    /// Application are attested by several attestors, and reach healthy state
    fn application_healthy(who: Self::AccountId) -> DispatchResult;
}

pub trait AttestorTrait {
    fn is_abnormal_mode() -> bool;
}
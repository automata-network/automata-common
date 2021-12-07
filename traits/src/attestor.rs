use primitives::attestor::ReportType;

pub trait ApplicationTrait {
    fn unhealthy_application(who: Self::AccountId, report_type: ReportType) -> DispatchResult;
}

pub trait AttestorTrait {

}
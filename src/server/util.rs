use iso14229_1::response::Response;
use iso14229_1::{response::Code, DidConfig, Service};

#[inline(always)]
pub fn service_not_support(service: u8) -> Vec<u8> {
    vec![
        Service::NRC.into(),
        service,
        Code::ServiceNotSupported.into(),
    ]
}

#[inline(always)]
pub fn service_not_support_in_session(service: Service) -> Vec<u8> {
    vec![
        Service::NRC.into(),
        service.into(),
        Code::ServiceNotSupportedInActiveSession.into(),
    ]
}

#[inline(always)]
pub fn sub_func_not_support(service: Service) -> Vec<u8> {
    vec![
        Service::NRC.into(),
        service.into(),
        Code::SubFunctionNotSupported.into(),
    ]
}

#[inline(always)]
pub fn positive_response(
    service: Service,
    sub_func: Option<u8>,
    data: Vec<u8>,
    cfg: &DidConfig,
) -> Vec<u8> {
    match Response::new(service, sub_func, data, &cfg) {
        Ok(v) => v.into(),
        Err(_) => vec![
            Service::NRC.into(),
            service.into(),
            Code::GeneralReject.into(),
        ],
    }
}

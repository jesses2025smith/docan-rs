use iso14229_1::response::Response;
use iso14229_1::{response::Code, DidConfig, Service};
use rand::{rng, Rng};

#[inline(always)]
pub fn sub_func_not_support(service: Service) -> Vec<u8> {
    vec![
        Service::NRC.into(),
        service.into(),
        Code::SubFunctionNotSupported.into(),
    ]
}

#[inline(always)]
pub fn invalid_format(service: Service) -> Vec<u8> {
    vec![
        Service::NRC.into(),
        service.into(),
        Code::IncorrectMessageLengthOrInvalidFormat.into(),
    ]
}

#[inline(always)]
pub fn positive_response<T: AsRef<[u8]>>(
    service: Service,
    sub_func: Option<u8>,
    data: T,
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

#[inline(always)]
pub fn gen_seed(num: usize) -> Vec<u8> {
    let mut rng = rng();
    let mut res = Vec::new();
    for _ in 0..num {
        res.push(rng.random::<u8>());
    }

    res
}

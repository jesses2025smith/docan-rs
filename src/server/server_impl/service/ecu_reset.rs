//! request of Service 11

use crate::server::{util, DoCanServer};
use iso14229_1::{request::Request, DidConfig, ECUResetType, Service};

impl<D, C, F> DoCanServer<D, C, F> {
    pub(crate) fn ecu_reset(&self, req: Request, cfg: &DidConfig) -> Option<Vec<u8>> {
        match req.sub_function() {
            Some(sub_func) => {
                if sub_func.is_suppress_positive() {
                    None
                } else {
                    let sub_func: ECUResetType = sub_func.function().unwrap();
                    let data = match sub_func {
                        ECUResetType::EnableRapidPowerShutDown => vec![1],
                        _ => vec![],
                    };

                    Some(util::positive_response(
                        Service::ECUReset,
                        Some(sub_func.into()),
                        data,
                        cfg,
                    ))
                }
            }
            None => Some(util::sub_func_not_support(Service::ECUReset.into())),
        }
    }
}

//! request of Service 10

use crate::server::{util, DoCanServer};
use iso14229_1::{request::Request, DidConfig, Service, SessionType};

impl<D, C, F> DoCanServer<D, C, F> {
    pub(crate) fn session_ctrl(&self, req: Request, cfg: &DidConfig) -> Option<Vec<u8>> {
        match req.sub_function() {
            Some(sub_func) => {
                if sub_func.is_suppress_positive() {
                    None
                } else {
                    let sub_func: SessionType = sub_func.function().unwrap();
                    let data: Vec<_> = self.context.timing.into();

                    Some(util::positive_response(
                        Service::SessionCtrl,
                        Some(sub_func.into()),
                        data,
                        cfg,
                    ))
                }
            }
            None => Some(util::sub_func_not_support(Service::SessionCtrl.into())),
        }
    }
}

//! request of Service 10

use crate::server::{util, DoCanServer};
use iso14229_1::{
    request::Request, response::Response, DidConfig, Iso14229Error, Service, SessionType,
};

impl<D, C, F> DoCanServer<D, C, F> {
    pub(crate) async fn session_ctrl(
        &self,
        req: Request,
        cfg: &DidConfig,
        data: Vec<u8>,
    ) -> Result<Response, Iso14229Error> {
        let data = match req.sub_function() {
            Some(sub_func) => {
                if sub_func.is_suppress_positive() {
                    util::sub_func_not_support(Service::SessionCtrl.into())
                } else {
                    match sub_func.function::<SessionType>() {
                        Ok(sub_func) => util::positive_response(
                            Service::SessionCtrl,
                            Some(sub_func.into()),
                            data,
                            cfg,
                        ),
                        Err(_) => util::sub_func_not_support(Service::SessionCtrl.into()),
                    }
                }
            }
            None => util::sub_func_not_support(Service::SessionCtrl.into()),
        };

        Response::try_from((&data, cfg))
    }
}

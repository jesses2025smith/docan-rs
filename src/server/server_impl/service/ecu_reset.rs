//! request of Service 11

use crate::server::{util, DoCanServer};
use iso14229_1::{
    request::Request, response::Response, DidConfig, ECUResetType, Iso14229Error, Service,
};

impl<D, C, F> DoCanServer<D, C, F> {
    pub(crate) async fn ecu_reset(
        &self,
        req: Request,
        cfg: &DidConfig,
    ) -> Result<Response, Iso14229Error> {
        let data = match req.sub_function() {
            Some(sub_func) => {
                if sub_func.is_suppress_positive() {
                    util::sub_func_not_support(Service::ECUReset.into())
                } else {
                    let data = match sub_func.function::<ECUResetType>() {
                        Ok(sub_func) => match sub_func {
                            ECUResetType::EnableRapidPowerShutDown => vec![1],
                            _ => vec![],
                        },
                        Err(_) => util::sub_func_not_support(Service::ECUReset.into()),
                    };

                    util::positive_response(Service::ECUReset, Some(sub_func.into()), data, cfg)
                }
            }
            None => util::sub_func_not_support(Service::ECUReset.into()),
        };

        Response::try_from((data, cfg))
    }
}

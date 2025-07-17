//! response of Service 11

use crate::{
    constants::LOG_TAG_SERVER,
    server::{util, DoCanServer},
};
use iso14229_1::{request::Request, response::Response, DidConfig, ECUResetType, Iso14229Error};
use rs_can::{CanDevice, CanFrame};
use std::fmt::Display;

impl<D, C, F> DoCanServer<D, C, F>
where
    D: CanDevice<Channel = C, Frame = F> + Clone + Send + Sync + 'static,
    C: Clone + Eq + Display + Send + Sync + 'static,
    F: CanFrame<Channel = C> + Clone + Display + Send + Sync + 'static,
{
    pub(crate) async fn ecu_reset(
        &self,
        req: Request,
        _cfg: &DidConfig,
    ) -> Result<(), Iso14229Error> {
        let service = req.service();
        let data = match req.sub_function() {
            Some(sf) => {
                match sf.function::<ECUResetType>() {
                    Ok(r#type) => {
                        self.context.reset().await;
                        if sf.is_suppress_positive() {
                            None // suppress positive
                        } else {
                            let data = match r#type {
                                ECUResetType::EnableRapidPowerShutDown => vec![1],
                                _ => vec![],
                            };
                            Some(util::positive_response(
                                service,
                                Some(r#type.into()),
                                data,
                                _cfg,
                            ))
                        }
                    }
                    Err(e) => {
                        rsutil::warn!("{} Failed to parse sub-function: {:?}", LOG_TAG_SERVER, e);
                        Some(util::sub_func_not_support(service))
                    }
                }
            }
            None => Some(util::invalid_format(service)),
        };

        if let Some(data) = data {
            self.transmit_response(Response::try_from((&data, _cfg))?, true)
                .await;
        }

        Ok(())
    }
}

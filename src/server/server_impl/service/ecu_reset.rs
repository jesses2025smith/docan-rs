//! request of Service 11

use crate::server::{util, DoCanServer};
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
        cfg: &DidConfig,
    ) -> Result<(), Iso14229Error> {
        let service = req.service();
        let data = match req.sub_function() {
            Some(sub_func) => {
                if sub_func.is_suppress_positive() {
                    None // suppress positive
                } else {
                    match sub_func.function::<ECUResetType>() {
                        Ok(sub_func) => {
                            let data = match sub_func {
                                ECUResetType::EnableRapidPowerShutDown => vec![1],
                                _ => vec![],
                            };
                            Some(util::positive_response(
                                service,
                                Some(sub_func.into()),
                                data,
                                cfg,
                            ))
                        }
                        Err(_) => Some(util::sub_func_not_support(service)),
                    }
                }
            }
            None => Some(util::sub_func_not_support(service)),
        };

        if let Some(data) = data {
            self.transmit_response(Response::try_from((&data, cfg))?)
                .await;
        }

        Ok(())
    }
}

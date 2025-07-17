//! response of Service 10

use crate::{
    constants::LOG_TAG_SERVER,
    server::{util, DoCanServer},
};
use iso14229_1::{request::Request, response::Response, DidConfig, Iso14229Error, SessionType};
use rs_can::{CanDevice, CanFrame};
use std::fmt::Display;

impl<D, C, F> DoCanServer<D, C, F>
where
    D: CanDevice<Channel = C, Frame = F> + Clone + Send + Sync + 'static,
    C: Clone + Eq + Display + Send + Sync + 'static,
    F: CanFrame<Channel = C> + Clone + Display + Send + Sync + 'static,
{
    pub(crate) async fn session_ctrl(
        &mut self,
        req: Request,
        _cfg: &DidConfig,
        data: Vec<u8>,
    ) -> Result<(), Iso14229Error> {
        let service = req.service();
        let data = match req.sub_function() {
            Some(sf) => {
                match sf.function::<SessionType>() {
                    Ok(r#type) => {
                        self.session.change(r#type).await;
                        if r#type != Default::default() {
                            self.session.keep().await;
                        }

                        if sf.is_suppress_positive() {
                            None // suppress positive
                        } else {
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

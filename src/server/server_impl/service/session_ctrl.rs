//! response of Service 10

use crate::{constants::LOG_TAG_SERVER, server::DoCanServer};
use iso14229_1::{
    request::Request,
    response::{Code, Response},
    DidConfig, Iso14229Error, SessionType,
};
use rs_can::{CanDevice, CanFrame};
use std::fmt::Display;

impl<D, C, F> DoCanServer<D, C, F>
where
    D: CanDevice<Channel = C, Frame = F> + Clone + Send + 'static,
    C: Clone + Eq + Display + Send + Sync + 'static,
    F: CanFrame<Channel = C> + Clone + Display + 'static,
{
    pub(crate) async fn session_ctrl(
        &mut self,
        req: Request,
        _cfg: &DidConfig,
        data: Vec<u8>,
    ) -> Result<(), Iso14229Error> {
        let service = req.service();
        let resp = match req.sub_function() {
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
                            Some(Response::new(service, Some(r#type.into()), data, _cfg)?)
                        }
                    }
                    Err(e) => {
                        rsutil::warn!("{} failed to parse sub-function: {:?}", LOG_TAG_SERVER, e);
                        Some(Response::new_negative(
                            service,
                            Code::IncorrectMessageLengthOrInvalidFormat,
                        ))
                    }
                }
            }
            None => Some(Response::new_negative(service, Code::GeneralReject)),
        };

        if let Some(resp) = resp {
            self.transmit_response(resp, true).await;
        }

        Ok(())
    }
}

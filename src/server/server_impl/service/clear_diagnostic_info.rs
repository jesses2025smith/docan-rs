//! response of Service 14

use crate::{constants::LOG_TAG_SERVER, server::DoCanServer};
use iso14229_1::{
    request::{self, Request},
    response::{Code, Response},
    DidConfig, Iso14229Error,
};
use rs_can::{CanDevice, CanFrame};
use std::fmt::Display;

impl<D, C, F> DoCanServer<D, C, F>
where
    D: CanDevice<Channel = C, Frame = F> + Clone + Send + Sync + 'static,
    C: Clone + Eq + Display + Send + Sync + 'static,
    F: CanFrame<Channel = C> + Clone + Display + Send + Sync + 'static,
{
    pub(crate) async fn clear_diagnostic_info(
        &self,
        req: Request,
        _cfg: &DidConfig,
    ) -> Result<(), Iso14229Error> {
        let service = req.service();

        let resp = match req.data::<request::ClearDiagnosticInfo>(_cfg) {
            Ok(info) => {
                self.context.clear_diagnostic_info(info).await;
                Response::new(service, None, vec![], _cfg)?
            }
            Err(e) => {
                rsutil::warn!(
                    "{} can't parse sub-function on service: {}, because of: {}",
                    LOG_TAG_SERVER,
                    service,
                    e
                );
                Response::new_negative(service, Code::IncorrectMessageLengthOrInvalidFormat)
            }
        };

        self.transmit_response(resp, true).await;

        Ok(())
    }
}

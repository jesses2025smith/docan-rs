//! response of Service 36

use crate::{constants::LOG_TAG_SERVER, server::DoCanServer};
use iso14229_1::{
    request::{self, Request},
    response::{self, Code, Response},
    DidConfig, Iso14229Error,
};
use rs_can::{CanDevice, CanFrame};
use std::fmt::Display;

impl<D, C, F> DoCanServer<D, C, F>
where
    D: CanDevice<Channel = C, Frame = F> + Clone + Send + 'static,
    C: Clone + Eq + Display + Send + Sync + 'static,
    F: CanFrame<Channel = C> + Clone + Display + 'static,
{
    pub(crate) async fn transfer_data(
        &self,
        req: Request,
        _cfg: &DidConfig,
    ) -> Result<(), Iso14229Error> {
        let service = req.service();
        let resp = match req.data::<request::TransferData>(_cfg) {
            Ok(v) => {
                let data: Vec<_> = response::TransferData {
                    sequence: v.sequence,
                    data: v.data,
                }
                .into();

                Response::new(service, None, data, _cfg)?
            }
            Err(e) => {
                rsutil::warn!("{} Failed to parse request data: {:?}", LOG_TAG_SERVER, e);
                Response::new_negative(service, Code::IncorrectMessageLengthOrInvalidFormat)
            }
        };

        self.transmit_response(resp, true).await;

        Ok(())
    }
}

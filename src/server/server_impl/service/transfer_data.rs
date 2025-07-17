//! response of Service 36

use crate::{
    constants::LOG_TAG_SERVER,
    server::{util, DoCanServer},
};
use iso14229_1::{
    request::{self, Request},
    response::{self, Response},
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
    pub(crate) async fn transfer_data(
        &self,
        req: Request,
        _cfg: &DidConfig,
    ) -> Result<(), Iso14229Error> {
        let service = req.service();
        let data = match req.data::<request::TransferData>(_cfg) {
            Ok(v) => response::TransferData {
                sequence: v.sequence,
                data: v.data,
            }
            .into(),
            Err(e) => {
                rsutil::warn!("{} Failed to parse request data: {:?}", LOG_TAG_SERVER, e);
                util::invalid_format(service)
            }
        };

        self.transmit_response(Response::try_from((&data, _cfg))?, true)
            .await;

        Ok(())
    }
}

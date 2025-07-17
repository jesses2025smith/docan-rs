//! response of Service 37

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
    pub(crate) async fn request_transfer_exit(
        &self,
        req: Request,
        _cfg: &DidConfig,
    ) -> Result<(), Iso14229Error> {
        let service = req.service();
        let data = match req.data::<request::RequestTransferExit>(_cfg) {
            Ok(ctx) => response::RequestTransferExit { data: ctx.data }.into(),
            Err(e) => {
                rsutil::warn!("{} Failed to parse request data: {:?}", LOG_TAG_SERVER, e);
                util::sub_func_not_support(service)
            }
        };

        self.transmit_response(Response::try_from((&data, _cfg))?, true)
            .await;

        Ok(())
    }
}

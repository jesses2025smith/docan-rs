//! request of Service 2F

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
    D: CanDevice<Channel = C, Frame = F> + Clone + Send + Sync + 'static,
    C: Clone + Eq + Display + Send + Sync + 'static,
    F: CanFrame<Channel = C> + Clone + Display + Send + Sync + 'static,
{
    pub(crate) async fn io_ctrl(&self, req: Request, cfg: &DidConfig) -> Result<(), Iso14229Error> {
        let service = req.service();
        let resp = match req.data::<request::IOCtrl>(cfg) {
            Ok(ctx) => {
                let data = response::IOCtrl::new(ctx.did, ctx.option.param, ctx.option.state);

                let data: Vec<_> = data.into();
                Response::new(service, None, data, cfg)?
            }
            Err(e) => {
                rsutil::warn!("{} Failed to parse request data: {:?}", LOG_TAG_SERVER, e);
                Response::new_negative(service, Code::GeneralReject)
            }
        };

        self.transmit_response(resp, true).await;

        Ok(())
    }
}

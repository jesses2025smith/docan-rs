//! request of Service 28

use crate::{constants::LOG_TAG_SERVER, server::DoCanServer};
use iso14229_1::{
    request::{CommunicationCtrl, Request},
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
    pub(crate) async fn communication_ctrl(
        &self,
        req: Request,
        _cfg: &DidConfig,
    ) -> Result<(), Iso14229Error> {
        let service = req.service();
        let resp = match req.data::<CommunicationCtrl>(_cfg) {
            Ok(ctx) => Response::try_from((service, vec![ctx.comm_type.value()], _cfg))?,
            Err(e) => {
                rsutil::warn!("{} Failed to parse request data: {:?}", LOG_TAG_SERVER, e);
                Response::new_negative(service, Code::GeneralReject)
            }
        };

        self.transmit_response(resp, true).await;

        Ok(())
    }
}

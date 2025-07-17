//! response of Service 86

use crate::server::DoCanServer;
use iso14229_1::{
    request::Request,
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
    pub(crate) async fn response_on_event(
        &self,
        req: Request,
        _cfg: &DidConfig,
    ) -> Result<(), Iso14229Error> {
        let service = req.service();

        self.transmit_response(
            Response::new_negative(service, Code::ServiceNotSupported),
            true,
        )
        .await;

        Ok(())
    }
}

//! request of Service 14

use crate::server::{util, DoCanServer};
use iso14229_1::{request::Request, response::Response, DidConfig, Iso14229Error};
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
        cfg: &DidConfig,
    ) -> Result<(), Iso14229Error> {
        let service = req.service();
        let data = util::positive_response(service, None, vec![], cfg);

        self.context.clear_diagnostic_info().await;

        self.transmit_response(Response::try_from((&data, cfg))?)
            .await;

        Ok(())
    }
}

//! request of Service 87

use crate::{
    constants::LOG_TAG_SERVER,
    server::{util, DoCanServer},
};
use iso14229_1::{request::Request, response::Response, DidConfig, Iso14229Error, LinkCtrlType};
use rs_can::{CanDevice, CanFrame};
use std::fmt::Display;

impl<D, C, F> DoCanServer<D, C, F>
where
    D: CanDevice<Channel = C, Frame = F> + Clone + Send + Sync + 'static,
    C: Clone + Eq + Display + Send + Sync + 'static,
    F: CanFrame<Channel = C> + Clone + Display + Send + Sync + 'static,
{
    pub(crate) async fn link_ctrl(
        &self,
        req: Request,
        _cfg: &DidConfig,
    ) -> Result<(), Iso14229Error> {
        let service = req.service();
        let data = match req.sub_function() {
            Some(sf) => match sf.function::<LinkCtrlType>() {
                Ok(r#type) => util::positive_response(service, Some(r#type.into()), vec![], _cfg),
                Err(e) => {
                    rsutil::warn!("{} Failed to parse sub-function: {:?}", LOG_TAG_SERVER, e);
                    util::sub_func_not_support(service)
                }
            },
            None => util::sub_func_not_support(service),
        };

        self.transmit_response(Response::try_from((&data, _cfg))?, true)
            .await;

        Ok(())
    }
}

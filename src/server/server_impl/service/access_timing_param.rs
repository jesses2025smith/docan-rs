//! request of Service 83

use crate::server::DoCanServer;
use iso14229_1::{request::Request, response::Response, DidConfig, Iso14229Error, Service};

impl<D, C, F> DoCanServer<D, C, F> {
    pub(crate) async fn access_timing_param(
        &self,
        req: Request,
        cfg: &DidConfig,
    ) -> Result<Response, Iso14229Error> {
        todo!()
    }
}

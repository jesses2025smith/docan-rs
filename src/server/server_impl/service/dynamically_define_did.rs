//! request of Service 2C

use crate::server::DoCanServer;
use iso14229_1::{request::Request, response::Response, DidConfig, Iso14229Error, Service};

impl<D, C, F> DoCanServer<D, C, F> {
    pub(crate) async fn dynamically_define_did(
        &self,
        req: Request,
        cfg: &DidConfig,
    ) -> Result<Response, Iso14229Error> {
        todo!()
    }
}

//! request of Service 29

use crate::server::DoCanServer;
use iso14229_1::{request::Request, response::Response, DidConfig, Iso14229Error, Service};

impl<D, C, F> DoCanServer<D, C, F> {
    pub(crate) async fn authentication(
        &self,
        req: Request,
        cfg: &DidConfig,
    ) -> Result<Response, Iso14229Error> {
        todo!()
    }
}

//! request of Service 3D

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
    pub(crate) async fn write_mem_by_addr(
        &self,
        req: Request,
        cfg: &DidConfig,
    ) -> Result<(), Iso14229Error> {
        todo!()
    }
}

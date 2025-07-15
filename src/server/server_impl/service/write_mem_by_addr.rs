//! request of Service 3D

use crate::server::DoCanServer;
use iso14229_1::{request::Request, DidConfig};

impl<D, C, F> DoCanServer<D, C, F> {
    pub(crate) fn write_mem_by_addr(&self, req: Request, cfg: &DidConfig) -> Option<Vec<u8>> {
        todo!()
    }
}

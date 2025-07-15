use crate::SecurityAlgo;
use iso14229_1::{response::SessionTiming, DidConfig};
use rsutil::types::ByteOrder;

#[derive(Default, Clone)]
pub struct Context {
    pub(crate) timing: SessionTiming,
    pub(crate) did_cfg: DidConfig,
    pub(crate) security_algo: Option<SecurityAlgo>,
    pub(crate) byte_order: ByteOrder, // session: SessionManager,
}

impl Context {}

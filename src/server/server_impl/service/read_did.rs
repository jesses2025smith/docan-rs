//! request of Service 22

use crate::{constants::LOG_TAG_SERVER, server::DoCanServer};
use iso14229_1::{
    request::{ReadDID, Request},
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
    pub(crate) async fn read_did(
        &self,
        req: Request,
        cfg: &DidConfig,
    ) -> Result<(), Iso14229Error> {
        let service = req.service();
        let resp = match req.data::<ReadDID>(cfg) {
            Ok(ctx) => {
                let list = ctx.collect();
                if list.is_empty() {
                    Response::new_negative(service, Code::GeneralReject)
                } else {
                    let mut data = Vec::with_capacity(list.len());
                    for did in list {
                        match self.context.get_static_did(&did).await {
                            Some(val) => {
                                let did_val: u16 = did.into();
                                data.extend_from_slice(did_val.to_be_bytes().as_slice());
                                data.extend_from_slice(val.as_ref());
                            }
                            None => {
                                rsutil::warn!(
                                    "{} DID: {:?} is not configured",
                                    LOG_TAG_SERVER,
                                    did
                                );
                                data.clear();
                                break;
                            }
                        }
                    }

                    if data.is_empty() {
                        Response::new_negative(service, Code::RequestOutOfRange)
                    } else {
                        Response::try_from((service, data, cfg))?
                    }
                }
            }
            Err(err) => {
                rsutil::warn!("{} Failed to parse request: {:?}", LOG_TAG_SERVER, err);
                Response::new_negative(service, Code::GeneralReject)
            }
        };

        self.transmit_response(resp).await;

        Ok(())
    }
}

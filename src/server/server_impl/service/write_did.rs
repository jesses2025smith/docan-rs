//! response of Service 2E

use crate::{constants::LOG_TAG_SERVER, server::DoCanServer};
use iso14229_1::{
    request::{Request, WriteDID},
    response::{Code, Response},
    DidConfig, Iso14229Error, SessionType,
};
use rs_can::{CanDevice, CanFrame};
use std::fmt::Display;

impl<D, C, F> DoCanServer<D, C, F>
where
    D: CanDevice<Channel = C, Frame = F> + Clone + Send + Sync + 'static,
    C: Clone + Eq + Display + Send + Sync + 'static,
    F: CanFrame<Channel = C> + Clone + Display + Send + Sync + 'static,
{
    pub(crate) async fn write_did(
        &mut self,
        req: Request,
        _cfg: &DidConfig,
    ) -> Result<(), Iso14229Error> {
        let service = req.service();
        let resp = match self.session.get_session_type().await {
            SessionType::Extended => {
                let sa_level = self.context.session.get_security_access_level().await;
                if self.context.config.extend_sa_level != sa_level {
                    // security access denied
                    Response::new_negative(service, Code::SecurityAccessDenied)
                } else {
                    match req.data::<WriteDID>(_cfg) {
                        Ok(ctx) => {
                            let did = ctx.0.did;
                            if self.context.set_static_did(&did, ctx.0.data).await {
                                let data: u16 = did.into();
                                Response::try_from((service, data.to_be_bytes(), _cfg))?
                            } else {
                                Response::new_negative(service, Code::GeneralReject)
                            }
                        }
                        Err(e) => {
                            rsutil::warn!(
                                "{} can't parse did context from data: {}",
                                LOG_TAG_SERVER,
                                e
                            );
                            Response::new_negative(
                                service,
                                Code::IncorrectMessageLengthOrInvalidFormat,
                            )
                        }
                    }
                }
            }
            _ => Response::new_negative(service, Code::ServiceNotSupportedInActiveSession),
        };

        self.transmit_response(resp, true).await;

        Ok(())
    }
}

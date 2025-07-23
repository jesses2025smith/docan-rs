//! response of Service 87

use crate::{constants::LOG_TAG_SERVER, server::DoCanServer};
use iso14229_1::{
    request::{self, Request},
    response::{Code, Response},
    DidConfig, Iso14229Error, LinkCtrlType, SessionType,
};
use rs_can::{CanDevice, CanFrame};
use std::fmt::Display;

impl<D, C, F> DoCanServer<D, C, F>
where
    D: CanDevice<Channel = C, Frame = F> + Clone + Send + 'static,
    C: Clone + Eq + Display + Send + Sync + 'static,
    F: CanFrame<Channel = C> + Clone + Display + 'static,
{
    pub(crate) async fn link_ctrl(
        &self,
        req: Request,
        _cfg: &DidConfig,
    ) -> Result<(), Iso14229Error> {
        let service = req.service();

        let resp = if self.session.get_session_type().await == SessionType::Default {
            Some(Response::new_negative(
                service,
                Code::ServiceNotSupportedInActiveSession,
            ))
        } else {
            match req.sub_function() {
                Some(sf) => match req.data::<request::LinkCtrl>(_cfg) {
                    Ok(_) => match sf.function::<LinkCtrlType>() {
                        Ok(r#type) => {
                            if sf.is_suppress_positive() {
                                None
                            } else {
                                Some(Response::new(service, Some(r#type.into()), vec![], _cfg)?)
                            }
                        }
                        Err(e) => {
                            rsutil::warn!(
                                "{} failed to parse request data: {:?}",
                                LOG_TAG_SERVER,
                                e
                            );
                            Some(Response::new_negative(
                                service,
                                Code::IncorrectMessageLengthOrInvalidFormat,
                            ))
                        }
                    },
                    Err(e) => {
                        rsutil::warn!("{} failed to parse request data: {:?}", LOG_TAG_SERVER, e);
                        Some(Response::new_negative(
                            service,
                            Code::IncorrectMessageLengthOrInvalidFormat,
                        ))
                    }
                },
                None => Some(Response::new_negative(service, Code::GeneralReject)),
            }
        };

        if let Some(resp) = resp {
            self.transmit_response(resp, true).await;
        }

        Ok(())
    }
}

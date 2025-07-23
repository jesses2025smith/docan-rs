//! response of Service 85

use crate::{constants::LOG_TAG_SERVER, server::DoCanServer};
use iso14229_1::{
    request::{self, Request},
    response::{Code, Response},
    DTCSettingType, DidConfig, Iso14229Error,
};
use rs_can::{CanDevice, CanFrame};
use std::fmt::Display;

impl<D, C, F> DoCanServer<D, C, F>
where
    D: CanDevice<Channel = C, Frame = F> + Clone + Send + 'static,
    C: Clone + Eq + Display + Send + Sync + 'static,
    F: CanFrame<Channel = C> + Clone + Display + 'static,
{
    pub(crate) async fn ctrl_dtc_setting(
        &self,
        req: Request,
        _cfg: &DidConfig,
    ) -> Result<(), Iso14229Error> {
        let service = req.service();

        let resp = if self.session.get_session_type().await == Default::default() {
            Some(Response::new_negative(
                service,
                Code::ServiceNotSupportedInActiveSession,
            ))
        } else {
            match req.sub_function() {
                Some(sf) => match req.data::<request::CtrlDTCSetting>(_cfg) {
                    Ok(_) => {
                        if sf.is_suppress_positive() {
                            None
                        } else {
                            match sf.function::<DTCSettingType>() {
                                Ok(r#type) => {
                                    Some(Response::new(service, Some(r#type.into()), vec![], _cfg)?)
                                }
                                Err(e) => {
                                    rsutil::warn!(
                                        "{} Failed to parse sub-function: {:?}",
                                        LOG_TAG_SERVER,
                                        e
                                    );
                                    Some(Response::new_negative(
                                        service,
                                        Code::SubFunctionNotSupported,
                                    ))
                                }
                            }
                        }
                    }
                    Err(e) => {
                        rsutil::warn!(
                            "{} can't parse data on service: {}, because of: {}",
                            LOG_TAG_SERVER,
                            service,
                            e
                        );
                        Some(Response::new_negative(
                            service,
                            Code::IncorrectMessageLengthOrInvalidFormat,
                        ))
                    }
                },
                None => {
                    rsutil::warn!(
                        "{} can't get sub-function on service: {}",
                        LOG_TAG_SERVER,
                        service
                    );
                    Some(Response::new_negative(service, Code::GeneralReject))
                }
            }
        };

        if let Some(resp) = resp {
            self.transmit_response(resp, true).await;
        }

        Ok(())
    }
}

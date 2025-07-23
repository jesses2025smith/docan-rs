//! response of Service 83

use crate::{constants::LOG_TAG_SERVER, server::DoCanServer};
use iso14229_1::{
    request::{self, Request},
    response::{Code, Response},
    DidConfig, Iso14229Error, TimingParameterAccessType,
};
use rs_can::{CanDevice, CanFrame};
use std::fmt::Display;

impl<D, C, F> DoCanServer<D, C, F>
where
    D: CanDevice<Channel = C, Frame = F> + Clone + Send + 'static,
    C: Clone + Eq + Display + Send + 'static,
    F: CanFrame<Channel = C> + Clone + Display + 'static,
{
    pub(crate) async fn access_timing_param(
        &self,
        req: Request,
        _cfg: &DidConfig,
    ) -> Result<(), Iso14229Error> {
        let service = req.service();

        let resp = if self.session.get_session_type() == Default::default() {
            Some(Response::new_negative(
                service,
                Code::ServiceNotSupportedInActiveSession,
            ))
        } else {
            match req.sub_function() {
                Some(sf) => match req.data::<request::AccessTimingParameter>(_cfg) {
                    Ok(data) => match sf.function::<TimingParameterAccessType>() {
                        Ok(r#type) => {
                            if sf.is_suppress_positive() {
                                None
                            } else {
                                Some(Response::new(service, Some(r#type.into()), data, _cfg)?)
                            }
                        }
                        Err(e) => {
                            rsutil::warn!(
                                "{} can't parse sub-function on service: {}, because of: {}",
                                LOG_TAG_SERVER,
                                service,
                                e
                            );
                            Some(Response::new_negative(
                                service,
                                Code::SubFunctionNotSupported,
                            ))
                        }
                    },
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

//! response of Service 31

use crate::{constants::LOG_TAG_SERVER, server::DoCanServer};
use iso14229_1::{
    request::{Request, RoutineCtrl},
    response::{Code, Response},
    DidConfig, Iso14229Error, RoutineCtrlType,
};
use rs_can::{CanDevice, CanFrame};
use std::fmt::Display;

impl<D, C, F> DoCanServer<D, C, F>
where
    D: CanDevice<Channel = C, Frame = F> + Clone + Send + Sync + 'static,
    C: Clone + Eq + Display + Send + Sync + 'static,
    F: CanFrame<Channel = C> + Clone + Display + Send + Sync + 'static,
{
    pub(crate) async fn routine_ctrl(
        &self,
        req: Request,
        _cfg: &DidConfig,
    ) -> Result<(), Iso14229Error> {
        let service = req.service();

        let resp = match req.data::<RoutineCtrl>(_cfg) {
            Ok(val) => match req.sub_function() {
                Some(sf) => {
                    if sf.is_suppress_positive() {
                        None // suppress positive
                    } else {
                        match sf.function::<RoutineCtrlType>() {
                            Ok(r#type) => {
                                let val: u16 = val.routine_id.into();
                                Some(Response::new(
                                    service,
                                    Some(r#type.into()),
                                    val.to_be_bytes(),
                                    _cfg,
                                )?)
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
                None => Some(Response::new_negative(service, Code::GeneralReject)),
            },
            Err(e) => {
                rsutil::warn!("{} failed to parse request data: {:?}", LOG_TAG_SERVER, e);
                Some(Response::new_negative(
                    service,
                    Code::IncorrectMessageLengthOrInvalidFormat,
                ))
            }
        };

        if let Some(resp) = resp {
            self.transmit_response(resp, true).await;
        }

        Ok(())
    }
}

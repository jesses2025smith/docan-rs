//! request of Service 31

use crate::{
    constants::LOG_TAG_SERVER,
    server::{util, DoCanServer},
};
use iso14229_1::{
    request::{Request, RoutineCtrl},
    response::Response,
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
        let data = match req.data::<RoutineCtrl>(_cfg) {
            Ok(val) => match req.sub_function() {
                Some(sf) => {
                    if sf.is_suppress_positive() {
                        None // suppress positive
                    } else {
                        match sf.function::<RoutineCtrlType>() {
                            Ok(r#type) => {
                                let val: u16 = val.routine_id.into();
                                Some(util::positive_response(
                                    service,
                                    Some(r#type.into()),
                                    val.to_be_bytes(),
                                    _cfg,
                                ))
                            }
                            Err(e) => {
                                rsutil::warn!(
                                    "{} Failed to parse sub-function: {:?}",
                                    LOG_TAG_SERVER,
                                    e
                                );
                                Some(util::sub_func_not_support(service))
                            }
                        }
                    }
                }
                None => Some(util::sub_func_not_support(service)),
            },
            Err(e) => {
                rsutil::warn!("{} Failed to parse request data: {:?}", LOG_TAG_SERVER, e);
                Some(util::sub_func_not_support(service))
            }
        };

        if let Some(data) = data {
            self.transmit_response(Response::try_from((&data, _cfg))?, true)
                .await;
        }

        Ok(())
    }
}

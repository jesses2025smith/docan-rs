//! response of Service 27

use crate::{
    constants::LOG_TAG_SERVER,
    server::{util, DoCanServer},
};
use bytes::Bytes;
use iso14229_1::response::Code;
use iso14229_1::{
    request::Request, response::Response, DidConfig, Iso14229Error, SecurityAccessLevel,
};
use rs_can::{CanDevice, CanFrame};
use std::fmt::Display;

impl<D, C, F> DoCanServer<D, C, F>
where
    D: CanDevice<Channel = C, Frame = F> + Clone + Send + Sync + 'static,
    C: Clone + Eq + Display + Send + Sync + 'static,
    F: CanFrame<Channel = C> + Clone + Display + Send + Sync + 'static,
{
    pub(crate) async fn security_access(
        &self,
        req: Request,
        _cfg: &DidConfig,
    ) -> Result<(), Iso14229Error> {
        let service = req.service();
        let resp = match req.sub_function() {
            Some(sf) => match sf.function::<SecurityAccessLevel>() {
                Ok(v) => {
                    let mut guard = self.context.sa_ctx.lock().await;
                    if v.is_request_seed() {
                        let data = util::gen_seed(4);
                        let resp = Response::new(service, Some(v.into()), &data, _cfg)?;
                        let _ = guard.replace((v.into(), Bytes::from(data)));

                        resp
                    } else {
                        match guard.take() {
                            Some(ctx) => {
                                let level = v.into();
                                if level - 1 != ctx.0 {
                                    Response::new_negative(service, Code::ConditionsNotCorrect)
                                } else {
                                    match self.context.get_security_algo().await {
                                        Some(algo) => {
                                            // TODO salt
                                            match algo(ctx.0, ctx.1.as_ref(), &vec![]) {
                                                Ok(v) => match v {
                                                    Some(v) => Response::new(
                                                        service,
                                                        Some(level),
                                                        v,
                                                        _cfg,
                                                    )?,
                                                    None => Response::new_negative(
                                                        service,
                                                        Code::SecurityAccessDenied,
                                                    ),
                                                },
                                                Err(e) => {
                                                    rsutil::warn!(
                                                        "{} error: {} when calculator sa key",
                                                        LOG_TAG_SERVER,
                                                        e
                                                    );
                                                    Response::new_negative(
                                                        service,
                                                        Code::GeneralReject,
                                                    )
                                                }
                                            }
                                        }
                                        None => Response::new_negative(
                                            service,
                                            Code::ConditionsNotCorrect,
                                        ),
                                    }
                                }
                            }
                            None => Response::new_negative(service, Code::SubFunctionNotSupported),
                        }
                    }
                }
                Err(e) => {
                    rsutil::warn!("{} Failed to parse access level: {:?}", LOG_TAG_SERVER, e);
                    Response::new_negative(service, Code::SubFunctionNotSupported)
                }
            },
            None => Response::new_negative(service, Code::IncorrectMessageLengthOrInvalidFormat),
        };

        self.transmit_response(resp, true).await;

        Ok(())
    }
}

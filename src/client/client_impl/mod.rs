mod trait_impl;

use crate::{client::context::Context, error::DoCanError};
use iso14229_1::{
    request::Request,
    response::{Code, Response},
    DidConfig, Service, TesterPresentType, SUPPRESS_POSITIVE,
};
use iso15765_2::{Address, AddressType, CanIsoTp, IsoTp, IsoTpError};
use rs_can::{CanDevice, CanFrame};
use rsutil::types::ByteOrder;
use std::{fmt::Display, hash::Hash};

#[derive(Clone)]
pub struct DoCanClient<D, C, F>
where
    D: CanDevice<Channel = C, Frame = F> + Clone + Send + Sync + 'static,
    C: Display + Clone + Hash + Eq + Send + Sync + 'static,
    F: CanFrame<Channel = C> + Clone + Display + Send + Sync + 'static,
{
    isotp: CanIsoTp<D, C, F>,
    context: Context,
}
unsafe impl<D, C, F> Send for DoCanClient<D, C, F>
where
    D: CanDevice<Channel = C, Frame = F> + Clone + Send + Sync + 'static,
    C: Display + Clone + Hash + Eq + Send + Sync + 'static,
    F: CanFrame<Channel = C> + Clone + Display + Send + Sync + 'static,
{
}
unsafe impl<D, C, F> Sync for DoCanClient<D, C, F>
where
    D: CanDevice<Channel = C, Frame = F> + Clone + Send + 'static,
    C: Display + Clone + Hash + Eq + Send + Sync + 'static,
    F: CanFrame<Channel = C> + Clone + Send + Display + 'static,
{
}

impl<D, C, F> DoCanClient<D, C, F>
where
    D: CanDevice<Channel = C, Frame = F> + Clone + Send + 'static,
    C: Display + Clone + Hash + Eq + Send + Sync + 'static,
    F: CanFrame<Channel = C> + Clone + Send + Display + 'static,
{
    pub async fn new(
        device: D,
        channel: C,
        addr: Address,
        byte_order: ByteOrder,
        p2_offset: Option<u16>,
    ) -> Self {
        Self {
            isotp: CanIsoTp::new(device, channel, addr, false).await,
            context: Context::new(p2_offset, byte_order),
        }
    }

    #[inline(always)]
    pub fn iso_tp(&mut self) -> &mut CanIsoTp<D, C, F> {
        &mut self.isotp
    }

    #[inline(always)]
    pub fn byte_order(&self) -> ByteOrder {
        self.context.byte_order
    }

    fn response_service_check(response: &Response, target: Service) -> Result<bool, DoCanError> {
        let service = response.service();
        if response.is_negative() {
            let nrc_code = response.nrc_code().map_err(DoCanError::ISO14229Error)?;
            match nrc_code {
                Code::RequestCorrectlyReceivedResponsePending => Ok(true),
                _ => Err(DoCanError::NRCError {
                    service,
                    code: nrc_code,
                }),
            }
        } else if service != target {
            Err(DoCanError::UnexpectedResponse {
                expect: target,
                actual: service,
            })
        } else {
            Ok(false)
        }
    }

    async fn suppress_positive_sr(
        &self,
        addr_type: AddressType,
        request: Request,
        suppress_positive: bool,
        cfg: &DidConfig,
    ) -> Result<Option<Response>, DoCanError> {
        match self.send_and_response(addr_type, request, cfg).await {
            Ok(r) => Ok(Some(r)),
            Err(e) => match e {
                DoCanError::IsoTpError(e) => match e {
                    IsoTpError::Timeout { .. } => {
                        if suppress_positive {
                            Ok(None)
                        } else {
                            Err(DoCanError::IsoTpError(e))
                        }
                    }
                    _ => Err(DoCanError::IsoTpError(e)),
                },
                _ => Err(e),
            },
        }
    }

    async fn send_and_response(
        &self,
        addr_type: AddressType,
        request: Request,
        cfg: &DidConfig,
    ) -> Result<Response, DoCanError> {
        let service = request.service();
        let data: Vec<_> = request.into();
        let timing = self.context.get_session_timing().await;
        let p2_offset = self.context.p2_offset;
        let _ = &self
            .isotp
            .transmit(addr_type, data)
            .await
            .map_err(DoCanError::IsoTpError)?;

        let data = &self
            .isotp
            .wait_data(timing.p2_ms() + p2_offset)
            .await
            .map_err(DoCanError::IsoTpError)?;
        let mut response = Response::try_from((data, cfg)).map_err(DoCanError::ISO14229Error)?;
        while Self::response_service_check(&response, service)? {
            rsutil::debug!(
                "DoCANClient - tester present when {:?}",
                Code::RequestCorrectlyReceivedResponsePending
            );
            let (_, request) =
                Self::tester_present_request(TesterPresentType::Zero, true, cfg).await?;
            let data: Vec<_> = request.into();
            let _ = &self
                .isotp
                .transmit(addr_type, data)
                .await
                .map_err(DoCanError::IsoTpError)?;

            let data = &self
                .isotp
                .wait_data(timing.p2_star_ms())
                .await
                .map_err(DoCanError::IsoTpError)?;

            response = Response::try_from((data, cfg)).map_err(DoCanError::ISO14229Error)?;
        }

        Ok(response)
    }

    fn sub_func_check(response: &Response, source: u8, service: Service) -> Result<(), DoCanError> {
        match response.sub_function() {
            Some(v) => {
                // let source: u8 = session_type.into();
                let target = v.origin();
                if target != source {
                    Err(DoCanError::UnexpectedSubFunction {
                        service,
                        expect: source,
                        actual: target,
                    })
                } else {
                    Ok(())
                }
            }
            None => Err(DoCanError::OtherError(format!(
                "response of service `{}` got an empty sub-function",
                service
            ))),
        }
    }

    #[inline(always)]
    async fn tester_present_request(
        test_type: TesterPresentType,
        suppress_positive: bool,
        did: &DidConfig,
    ) -> Result<(Service, Request), DoCanError> {
        let service = Service::TesterPresent;
        let mut sub_func = test_type.into();
        if suppress_positive {
            sub_func |= SUPPRESS_POSITIVE;
        }
        let request = Request::new(service, Some(sub_func), vec![], &did)
            .map_err(DoCanError::ISO14229Error)?;

        Ok((service, request))
    }
}

mod trait_impl;

use std::ops::Fn;
use crate::{
    client::context::{Context, IsoTpListener},
    error::DoCanError,
};
use iso14229_1::{
    request::Request,
    response::{Code, Response},
    Service, TesterPresentType, SUPPRESS_POSITIVE,
};
use iso15765_2::{Address, AddressType, CanAdapter, CanIsoTp, IsoTpError, IsoTpEventListener};
use rs_can::{CanDevice, CanFrame};
use std::{sync::Arc, collections::HashMap, fmt::Display, hash::Hash};
use rsutil::types::ByteOrder;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct DoCanClient<D, C, F>
where
    C: Clone + Eq,
{
    adapter: CanAdapter<D, C, F>,
    context: HashMap<C, Context<C, F>>,
    p2_offset: u64,
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
    pub fn new(adapter: CanAdapter<D, C, F>, p2_offset: Option<u16>) -> Self {
        Self {
            adapter,
            context: Default::default(),
            p2_offset: p2_offset.unwrap_or_default() as u64,
        }
    }

    pub async fn init_channel(
        &mut self,
        channel: C,
        address: Address,
        byte_order: ByteOrder,
    ) -> Result<(), DoCanError> {
        let listener = IsoTpListener::new(Default::default(), self.p2_offset);
        let iso_tp = CanIsoTp::new(
            channel.clone(),
            address,
            self.adapter.sender(),
            Box::new(listener.clone()),
        );

        self.adapter
            .register_listener(format!("DoCANClient-{}", channel), Box::new(iso_tp.clone()))
            .await;
        self.context.insert(
            channel,
            Context {
                iso_tp: Arc::new(Mutex::new(iso_tp)),
                listener: Arc::new(Mutex::new(listener)),
                did: Default::default(),
                security_algo: Default::default(),
                byte_order
            },
        );

        Ok(())
    }
    #[inline(always)]
    pub fn adapter(&self) -> &CanAdapter<D, C, F> {
        &self.adapter
    }

    #[inline]
    pub(crate) async fn context_util<R, Func, Fut>(
        &self,
        channel: C,
        callback: Func,
    ) -> Result<R, DoCanError>
    where
        Func: Fn(Context<C, F>) -> Fut+ Send,
        Fut: std::future::Future<Output=Result<R, DoCanError>>+ Send,
    {
        match self.context.get(&channel) {
            Some(ctx) => callback(ctx.clone()).await,
            None => Err(DoCanError::OtherError(format!(
                "channel: {} is not initialized",
                channel
            ))),
        }
    }

    pub(crate) fn response_service_check(
        response: &Response,
        target: Service,
    ) -> Result<bool, DoCanError> {
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

    pub(crate) async fn suppress_positive_sr(
        ctx: &Context<C, F>,
        addr_type: AddressType,
        request: Request,
        suppress_positive: bool,
    ) -> Result<Option<Response>, DoCanError> {
        match Self::send_and_response(&ctx, addr_type, request).await {
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

    pub(crate) async fn send_and_response(
        ctx: &Context<C, F>,
        addr_type: AddressType,
        request: Request,
    ) -> Result<Response, DoCanError> {
        ctx.listener.lock().await.clear_buffer().await;
        let service = request.service();
        ctx.iso_tp
            .lock()
            .await
            .write(addr_type, request.into())
            .await
            .map_err(DoCanError::IsoTpError)?;

        let data = ctx
            .listener
            .lock()
            .await
            .async_timer(false)
            .await
            .map_err(DoCanError::IsoTpError)?;
        let config = ctx.did.lock().await;
        let mut response =
            Response::try_from((data, &*config)).map_err(DoCanError::ISO14229Error)?;
        while Self::response_service_check(&response, service)? {
            rsutil::debug!(
                "DoCANClient - tester present when {:?}",
                Code::RequestCorrectlyReceivedResponsePending
            );
            let (_, request) = Self::tester_present_request(&ctx, TesterPresentType::Zero, true).await?;
            ctx.iso_tp
                .lock()
                .await
                .write(addr_type, request.into())
                .await
                .map_err(DoCanError::IsoTpError)?;

            let data = ctx
                .listener
                .lock()
                .await
                .async_timer(true)
                .await
                .map_err(DoCanError::IsoTpError)?;

            response =
                Response::try_from((data,&*config)).map_err(DoCanError::ISO14229Error)?;
        }

        Ok(response)
    }

    pub(crate) fn sub_func_check(
        response: &Response,
        source: u8,
        service: Service,
    ) -> Result<(), DoCanError> {
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

    #[inline]
    pub(crate) async fn tester_present_request(
        ctx: &Context<C, F>,
        test_type: TesterPresentType,
        suppress_positive: bool,
    ) -> Result<(Service, Request), DoCanError> {
        let service = Service::TesterPresent;
        let mut sub_func = test_type.into();
        if suppress_positive {
            sub_func |= SUPPRESS_POSITIVE;
        }
        let request = Request::new(service, Some(sub_func), vec![], &*ctx.did.lock().await)
            .map_err(DoCanError::ISO14229Error)?;

        Ok((service, request))
    }
}

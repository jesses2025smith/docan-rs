mod service;

use crate::{
    constants::LOG_TAG_SERVER,
    server::{context, session::SessionManager},
    DoCanError, SecurityAlgo, Server,
};
use iso14229_1::{
    request::Request,
    response::{Code, Response},
    Iso14229Error, Service,
};
use iso15765_2::{
    can::{Address, AddressType, CanIsoTp},
    IsoTp, IsoTpError,
};
use rs_can::{CanDevice, CanFrame};
use std::{fmt::Display, sync::Arc};
use tokio::{spawn, task::JoinHandle};

#[derive(Clone)]
pub struct DoCanServer<D, C, F> {
    isotp: CanIsoTp<D, C, F>,
    session: SessionManager,
    context: context::Context,
    handles: Vec<Arc<JoinHandle<()>>>,
}

impl<D, C, F> DoCanServer<D, C, F>
where
    D: CanDevice<Channel = C, Frame = F> + Clone + Send + 'static,
    C: Clone + Eq + Display + Send + Sync + 'static,
    F: CanFrame<Channel = C> + Clone + Display + 'static,
{
    pub async fn new(device: D, channel: C) -> Result<Self, DoCanError> {
        let context = context::Context::new().await?;
        Ok(Self {
            isotp: CanIsoTp::new(device, channel, context.config.address, true).await,
            session: SessionManager::new(None),
            context,
            handles: Default::default(),
        })
    }

    #[inline(always)]
    pub fn tp_layer(&mut self) -> CanIsoTp<D, C, F> {
        self.isotp.clone()
    }

    async fn server(&mut self) {
        loop {
            let timing = self.context.get_timing().clone();
            let cfg = self.context.get_did_config().clone();
            if let Ok(data) = self.isotp.wait_data(timing.p2_ms()).await {
                // rsutil::info!("{} Received data: {}", LOG_TAG_SERVER, hex::encode(&data));
                match data.len() {
                    0 => {}
                    _ => match Service::try_from(data[0]) {
                        Ok(service) => match Request::try_from((service, &data[1..], &cfg)) {
                            Ok(req) => {
                                if let Err(e) = match service {
                                    Service::SessionCtrl => {
                                        self.session_ctrl(req, &cfg, timing.into()).await
                                    }
                                    Service::ECUReset => self.ecu_reset(req, &cfg).await,
                                    Service::ClearDiagnosticInfo => {
                                        self.clear_diagnostic_info(req, &cfg).await
                                    }
                                    Service::ReadDTCInfo => self.read_dtc_info(req, &cfg).await,
                                    Service::ReadDID => self.read_did(req, &cfg).await,
                                    Service::ReadMemByAddr => {
                                        self.read_mem_by_addr(req, &cfg).await
                                    }
                                    Service::ReadScalingDID => {
                                        self.read_scaling_did(req, &cfg).await
                                    }
                                    Service::SecurityAccess => {
                                        self.security_access(req, &cfg).await
                                    }
                                    Service::CommunicationCtrl => {
                                        self.communication_ctrl(req, &cfg).await
                                    }
                                    #[cfg(any(feature = "std2020"))]
                                    Service::Authentication => self.authentication(req, &cfg).await,
                                    Service::ReadDataByPeriodId => {
                                        self.read_data_by_pid(req, &cfg).await
                                    }
                                    Service::DynamicalDefineDID => {
                                        self.dynamically_define_did(req, &cfg).await
                                    }
                                    Service::WriteDID => self.write_did(req, &cfg).await,
                                    Service::IOCtrl => self.io_ctrl(req, &cfg).await,
                                    Service::RoutineCtrl => self.routine_ctrl(req, &cfg).await,
                                    Service::RequestDownload => {
                                        self.request_download(req, &cfg).await
                                    }
                                    Service::RequestUpload => self.request_upload(req, &cfg).await,
                                    Service::TransferData => self.transfer_data(req, &cfg).await,
                                    Service::RequestTransferExit => {
                                        self.request_transfer_exit(req, &cfg).await
                                    }
                                    #[cfg(any(feature = "std2013", feature = "std2020"))]
                                    Service::RequestFileTransfer => {
                                        self.request_file_transfer(req, &cfg).await
                                    }
                                    Service::WriteMemByAddr => {
                                        self.write_mem_by_addr(req, &cfg).await
                                    }
                                    Service::TesterPresent => self.tester_present(req, &cfg).await,
                                    #[cfg(any(feature = "std2006", feature = "std2013"))]
                                    Service::AccessTimingParam => {
                                        self.access_timing_param(req, &cfg).await
                                    }
                                    Service::SecuredDataTrans => {
                                        self.secured_data_trans(req, &cfg).await
                                    }
                                    Service::CtrlDTCSetting => {
                                        self.ctrl_dtc_setting(req, &cfg).await
                                    }
                                    Service::ResponseOnEvent => {
                                        self.response_on_event(req, &cfg).await
                                    }
                                    Service::LinkCtrl => self.link_ctrl(req, &cfg).await,
                                    Service::NRC => {
                                        self.negative_service(
                                            Service::NRC.into(),
                                            Code::ServiceNotSupported,
                                        )
                                        .await;
                                        Ok(())
                                    }
                                } {
                                    self.process_uds_error(service, e).await;
                                }
                            }
                            Err(e) => {
                                rsutil::warn!(
                                    "{} error: {} when data: {} to request",
                                    LOG_TAG_SERVER,
                                    e,
                                    hex::encode(&data)
                                );
                                self.process_uds_error(service, e).await;
                            }
                        },
                        Err(_) => {
                            // can't parse service
                            self.negative_service(data[0], Code::ServiceNotSupported)
                                .await
                        }
                    },
                }
            }
        }
    }

    async fn negative_service(&self, service: u8, code: Code) {
        let data = vec![Service::NRC.into(), service, code.into()];
        if let Err(e) = self.isotp.transmit(AddressType::Physical, data).await {
            rsutil::error!(
                "{} can't transmit negative response, because of: {}",
                LOG_TAG_SERVER,
                e
            );
        }
    }

    async fn process_uds_error(&self, service: Service, e: Iso14229Error) {
        let code = match e {
            // Iso14229Error::InvalidParam(_) => {}
            // Iso14229Error::InvalidData(_) => {}
            Iso14229Error::InvalidDataLength { .. } => Code::IncorrectMessageLengthOrInvalidFormat,
            // Iso14229Error::DidNotSupported(_) => {}
            // Iso14229Error::InvalidDynamicallyDefinedDID(_) => {}
            // Iso14229Error::InvalidSessionData(_) => {}
            // Iso14229Error::ReservedError(_) => {}
            // Iso14229Error::SubFunctionError(_) => {}
            Iso14229Error::ServiceError(_) => Code::ConditionsNotCorrect,
            // Iso14229Error::OtherError(_) => {}
            // Iso14229Error::NotImplement => {}
            _ => Code::GeneralReject, // TODO
        };
        self.transmit_response(Response::new_negative(service, code), true)
            .await;
    }

    pub(crate) async fn transmit_response(&self, resp: Response, flag: bool) {
        let service = resp.service();
        let data: Vec<_> = resp.into();
        if let Err(e) = self.isotp.transmit(AddressType::Physical, data).await {
            rsutil::warn!("{} transmit error: {:?}", LOG_TAG_SERVER, e);
            if !flag {
                // resend negative response is no-need
                return;
            }

            if let Some(code) = match e {
                // IsoTpError::DeviceError => {}
                IsoTpError::EmptyPdu => Some(Code::IncorrectMessageLengthOrInvalidFormat),
                IsoTpError::InvalidPdu(_) => Some(Code::GeneralReject),
                IsoTpError::InvalidParam(_) => Some(Code::GeneralReject),
                IsoTpError::InvalidDataLength { .. } => {
                    Some(Code::IncorrectMessageLengthOrInvalidFormat)
                }
                IsoTpError::LengthOutOfRange(_) => Some(Code::RequestOutOfRange),
                IsoTpError::InvalidStMin(_) => Some(Code::GeneralReject),
                IsoTpError::InvalidSequence { .. } => Some(Code::WrongBlockSequenceCounter),
                IsoTpError::MixFramesError => Some(Code::GeneralReject),
                IsoTpError::Timeout { .. } => Some(Code::GeneralReject),
                IsoTpError::OverloadFlow => Some(Code::RequestOutOfRange),
                _ => None,
            } {
                let resp = Response::new_negative(service, code);
                Box::pin(self.transmit_response(resp, false)).await;
            }
        }
    }
}

#[async_trait::async_trait]
impl<D, C, F> Server for DoCanServer<D, C, F>
where
    D: CanDevice<Channel = C, Frame = F> + Clone + Send + 'static,
    C: Clone + Eq + Display + Send + Sync + 'static,
    F: CanFrame<Channel = C> + Clone + Display + 'static,
{
    #[inline(always)]
    async fn update_address(&self, address: Address) {
        self.isotp.update_address(address).await;
    }

    #[inline(always)]
    async fn update_security_algo(&self, algo: SecurityAlgo) {
        self.context.set_security_algo(algo).await;
    }

    async fn service_forever(&mut self, interval_us: u64) {
        self.isotp.start(interval_us).await;
        let mut clone = self.clone();
        let session = self.session.clone();
        let handle = spawn(async move { session.work().await });
        self.handles.push(Arc::new(handle));
        let handle = spawn(async move { clone.server().await });
        self.handles.push(Arc::new(handle));
    }

    async fn service_stop(&mut self) {
        self.isotp.stop().await;
        for handle in &self.handles {
            handle.abort();
        }
        rsutil::info!("{} stopped", LOG_TAG_SERVER);
    }
}

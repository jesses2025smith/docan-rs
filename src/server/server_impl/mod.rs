mod service;

use crate::{
    server::{context, tasks::session::SessionManager, util},
    Server,
};
use iso14229_1::{
    request::Request,
    response::{Code, Response},
    DidConfig, Service,
};
use iso15765_2::{Address, AddressType, CanIsoTp, IsoTp};
use rs_can::{CanDevice, CanFrame};
use rsutil::types::ByteOrder;
use std::{fmt::Display, sync::Arc};
use tokio::{spawn, task::JoinHandle};

#[derive(Clone)]
pub struct DoCanServer<D, C, F> {
    isotp: CanIsoTp<D, C, F>,
    session: SessionManager,
    context: context::Context,
    handle: Option<Arc<JoinHandle<()>>>,
}

impl<D, C, F> DoCanServer<D, C, F>
where
    D: CanDevice<Channel = C, Frame = F> + Clone + Send + Sync + 'static,
    C: Clone + Eq + Display + Send + Sync + 'static,
    F: CanFrame<Channel = C> + Clone + Display + Send + Sync + 'static,
{
    pub async fn new(device: D, channel: C, addr: Address, byte_order: ByteOrder) -> Self {
        Self {
            isotp: CanIsoTp::new(device, channel, addr, true).await,
            session: SessionManager::new(None),
            context: context::Context::default(),
            handle: None,
        }
    }

    pub(crate) async fn server(&self) {
        loop {
            let timing = self.context.timing.clone();
            let cfg = self.context.did_cfg.clone();
            if let Ok(data) = self.isotp.wait_data(timing.p2_ms()).await {
                match Request::try_from((data, &cfg)) {
                    Ok(req) => {
                        let service = req.service();
                        let data = match service {
                            Service::SessionCtrl => self.session_ctrl(req, &cfg),
                            Service::ECUReset => self.ecu_reset(req, &cfg),
                            Service::ClearDiagnosticInfo => self.clear_diagnostic_info(req, &cfg),
                            Service::ReadDTCInfo => self.read_dtc_info(req, &cfg),
                            Service::ReadDID => self.read_did(req, &cfg),
                            Service::ReadMemByAddr => self.read_mem_by_addr(req, &cfg),
                            Service::ReadScalingDID => self.read_scaling_did(req, &cfg),
                            Service::SecurityAccess => self.security_access(req, &cfg),
                            Service::CommunicationCtrl => self.communication_ctrl(req, &cfg),
                            #[cfg(any(feature = "std2020"))]
                            Service::Authentication => self.authentication(req, &cfg),
                            Service::ReadDataByPeriodId => self.read_data_by_pid(req, &cfg),
                            Service::DynamicalDefineDID => self.dynamically_define_did(req, &cfg),
                            Service::WriteDID => self.write_did(req, &cfg),
                            Service::IOCtrl => self.io_ctrl(req, &cfg),
                            Service::RoutineCtrl => self.routine_ctrl(req, &cfg),
                            Service::RequestDownload => self.request_download(req, &cfg),
                            Service::RequestUpload => self.request_upload(req, &cfg),
                            Service::TransferData => self.transfer_data(req, &cfg),
                            Service::RequestTransferExit => self.request_transfer_exit(req, &cfg),
                            #[cfg(any(feature = "std2013", feature = "std2020"))]
                            Service::RequestFileTransfer => self.request_file_transfer(req, &cfg),
                            Service::WriteMemByAddr => self.write_mem_by_addr(req, &cfg),
                            Service::TesterPresent => self.tester_present(req, &cfg),
                            #[cfg(any(feature = "std2006", feature = "std2013"))]
                            Service::AccessTimingParam => self.access_timing_param(req, &cfg),
                            Service::SecuredDataTrans => self.secured_data_trans(req, &cfg),
                            Service::CtrlDTCSetting => self.ctrl_dtc_setting(req, &cfg),
                            Service::ResponseOnEvent => self.response_on_event(req, &cfg),
                            Service::LinkCtrl => self.link_ctrl(req, &cfg),
                            Service::NRC => Some(util::service_not_support(Service::NRC.into())),
                        };

                        self.process_response(data, service, &cfg).await;
                    }
                    Err(err) => self.error_handler(err).await,
                }
            }
        }
    }

    async fn process_response(&self, data: Option<Vec<u8>>, service: Service, cfg: &DidConfig) {
        match data {
            Some(data) => {
                let resp = match Response::try_from((data, cfg)) {
                    Ok(resp) => resp,
                    Err(_) => {
                        let data = vec![
                            Service::NRC.into(),
                            service.into(),
                            Code::GeneralReject.into(),
                        ];
                        Response::try_from((data, cfg)).unwrap()
                    }
                };
                let data: Vec<_> = resp.into();
                if let Err(e) = self.isotp.transmit(AddressType::Physical, data).await {
                    rsutil::warn!("DoCanServer - transmit error: {:?}", e);
                }
            }
            None => {
                todo!()
            }
        }
    }
}

#[async_trait::async_trait]
impl<D, C, F> Server for DoCanServer<D, C, F>
where
    D: CanDevice<Channel = C, Frame = F> + Clone + Send + Sync + 'static,
    C: Clone + Eq + Display + Send + Sync + 'static,
    F: CanFrame<Channel = C> + Clone + Display + Send + Sync + 'static,
{
    async fn service_forever(&mut self, interval: u64) {
        self.isotp.start(interval).await;
        let clone = self.clone();
        let handle = spawn(async move { clone.server().await });
        self.handle.replace(Arc::new(handle));
    }

    async fn service_stop(&mut self) {
        if let Some(handle) = self.handle.take() {
            handle.abort();
        }
    }
}

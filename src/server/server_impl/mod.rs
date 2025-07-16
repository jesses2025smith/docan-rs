mod service;

use crate::{
    server::{context, tasks::session::SessionManager, util},
    SecurityAlgo, Server,
};
use iso14229_1::{
    request::Request,
    response::{Code, Response},
    DataIdentifier, DidConfig, Iso14229Error, Service,
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
    handles: Vec<Arc<JoinHandle<()>>>,
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
            context: context::Context::new(byte_order),
            handles: Default::default(),
        }
    }

    #[inline(always)]
    pub fn tp_layer(&mut self) -> CanIsoTp<D, C, F> {
        self.isotp.clone()
    }

    pub(crate) async fn server(&mut self) {
        loop {
            let timing = self.context.get_timing().await;
            let cfg = self.context.get_did_config().await;
            if let Ok(data) = self.isotp.wait_data(timing.p2_ms()).await {
                // rsutil::info!("DoCanServer - Received data: {}", hex::encode(&data));
                match data.len() {
                    0 => {}
                    _ => match Service::try_from(data[0]) {
                        Ok(service) => match Request::try_from((service, &data[1..], &cfg)) {
                            Ok(req) => {
                                let data = match service {
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
                                        let data = util::service_not_support(Service::NRC.into());
                                        Response::try_from((data, &cfg))
                                    }
                                };

                                self.process_response(service, data, &cfg).await;
                            }
                            Err(e) => {
                                rsutil::warn!(
                                    "DoCanServer - error: {} when data: {} to request",
                                    e,
                                    hex::encode(&data)
                                );
                                self.transmit_response(Response::new_negative(
                                    service,
                                    Code::GeneralReject,
                                ))
                                .await;
                            }
                        },
                        Err(_) => {
                            if let Ok(resp) =
                                Response::try_from((util::service_not_support(data[0]), &cfg))
                            {
                                self.transmit_response(resp).await;
                            }
                        }
                    },
                }

                match Request::try_from((data, &cfg)) {
                    Ok(req) => {
                        let service = req.service();
                    }
                    Err(err) => {}
                }
            }
        }
    }

    async fn process_response(
        &self,
        service: Service,
        data: Result<Response, Iso14229Error>,
        cfg: &DidConfig,
    ) {
        let resp = match data {
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
        self.transmit_response(resp).await;
    }

    #[inline(always)]
    async fn transmit_response(&self, resp: Response) {
        let data: Vec<_> = resp.into();
        if let Err(e) = self.isotp.transmit(AddressType::Physical, data).await {
            rsutil::warn!("DoCanServer - transmit error: {:?}", e);
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
    #[inline(always)]
    async fn update_address(&self, address: Address) {
        self.isotp.update_address(address).await;
    }

    #[inline(always)]
    async fn update_security_algo(&self, algo: SecurityAlgo) {
        self.context.set_security_algo(algo).await;
    }

    #[inline(always)]
    async fn add_data_identifier(&self, did: DataIdentifier, length: usize) {
        self.context.add_did(did, length).await;
    }

    #[inline(always)]
    async fn remove_data_identifier(&self, did: DataIdentifier) {
        self.context.remove_did(&did).await;
    }

    async fn service_forever(&mut self, interval: u64) {
        self.isotp.start(interval).await;
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
        rsutil::info!("DoCanServer - stopped");
    }
}

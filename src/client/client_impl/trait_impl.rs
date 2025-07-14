use crate::{
    client::{Client, DoCanClient},
    error::DoCanError,
    SecurityAlgo,
};
use iso14229_1::{
    request::{self, Request},
    response,
    utils::U24,
    *,
};
use iso15765_2::{Address, AddressType};
use rs_can::{CanDevice, CanFrame, CanResult};
use std::{fmt::Display, hash::Hash, time::Duration};
use tokio::time::sleep;

#[async_trait::async_trait]
impl<D, C, F> Client for DoCanClient<D, C, F>
where
    D: CanDevice<Channel = C, Frame = F> + Clone + Send + Sync + 'static,
    C: Display + Clone + Hash + Eq + Send + Sync + 'static,
    F: CanFrame<Channel = C> + Clone + Display + Send + Sync + 'static,
{
    type Channel = C;
    type Error = DoCanError;

    #[inline(always)]
    fn channel(&self) -> Self::Channel {
        self.isotp.get_channel()
    }

    async fn update_address(&self, address: Address) {
        self.isotp.update_address(address).await;
    }

    async fn update_security_algo(&self, algo: SecurityAlgo) {
        self.context.set_security_algo(algo).await;
    }

    async fn add_data_identifier(&self, did: DataIdentifier, length: usize) {
        self.context.add_did(did, length).await;
    }

    async fn remove_data_identifier(&self, did: DataIdentifier) {
        self.context.remove_did(&did).await;
    }

    async fn session_ctrl(
        &mut self,
        r#type: SessionType,
        suppress_positive: bool,
        addr_type: AddressType,
    ) -> CanResult<(), Self::Error> {
        let service = Service::SessionCtrl;
        let mut sub_func: u8 = r#type.into();
        if suppress_positive {
            sub_func |= SUPPRESS_POSITIVE;
        }
        let cfg = self.context.get_did_cfg().await;
        let request = Request::new(service, Some(sub_func), vec![], &cfg)
            .map_err(DoCanError::ISO14229Error)?;

        let timing = match self
            .suppress_positive_sr(addr_type, request, suppress_positive, &cfg)
            .await?
        {
            Some(resp) => {
                Self::sub_func_check(&resp, r#type.into(), service)?;

                Some(
                    resp.data::<response::SessionCtrl>(&cfg)
                        .map_err(DoCanError::ISO14229Error)?
                        .0,
                )
            }
            None => None,
        };

        if let Some(timing) = timing {
            self.context.set_session_timing(timing).await;
        }

        Ok(())
    }

    async fn ecu_reset(
        &mut self,
        r#type: ECUResetType,
        suppress_positive: bool,
        addr_type: AddressType,
    ) -> CanResult<(), Self::Error> {
        let service = Service::ECUReset;
        let mut sub_func: u8 = r#type.into();
        if suppress_positive {
            sub_func |= SUPPRESS_POSITIVE;
        }
        let cfg = self.context.get_did_cfg().await;
        let request = Request::new(service, Some(sub_func), vec![], &cfg)
            .map_err(DoCanError::ISO14229Error)?;

        if let Some(response) = self
            .suppress_positive_sr(addr_type, request, suppress_positive, &cfg)
            .await?
        {
            Self::sub_func_check(&response, r#type.into(), service)?;

            let resp = response
                .data::<response::ECUReset>(&cfg)
                .map_err(DoCanError::ISO14229Error)?;
            if let Some(seconds) = resp.second {
                sleep(Duration::from_secs(seconds as u64)).await;
            }
        }

        Ok(())
    }

    async fn security_access(
        &mut self,
        level: u8,
        params: Vec<u8>,
    ) -> CanResult<Vec<u8>, Self::Error> {
        let service = Service::SecurityAccess;
        let cfg = self.context.get_did_cfg().await;
        let request =
            Request::new(service, Some(level), params, &cfg).map_err(DoCanError::ISO14229Error)?;

        let response = self
            .send_and_response(AddressType::Physical, request, &cfg)
            .await?;

        Self::sub_func_check(&response, level, service)?;

        Ok(response.raw_data().to_vec())
    }

    async fn unlock_security_access(
        &mut self,
        level: u8,
        params: Vec<u8>,
        salt: Vec<u8>,
    ) -> CanResult<(), Self::Error> {
        let service = Service::SecurityAccess;
        let cfg = self.context.get_did_cfg().await;
        let req =
            Request::new(service, Some(level), params, &cfg).map_err(DoCanError::ISO14229Error)?;

        let resp = self
            .send_and_response(AddressType::Physical, req, &cfg)
            .await?;
        Self::sub_func_check(&resp, level, service)?;

        let seed = resp.raw_data().to_vec();
        let algo = self
            .context
            .get_security_algo()
            .await
            .ok_or_else(|| DoCanError::OtherError("security algorithm required".into()))?;
        match algo(level, seed, salt)? {
            Some(data) => {
                let request = Request::new(service, Some(level + 1), data, &cfg)
                    .map_err(DoCanError::ISO14229Error)?;
                let response = self
                    .send_and_response(AddressType::Physical, request, &cfg)
                    .await?;

                Self::sub_func_check(&response, level + 1, service)
            }
            None => Ok(()),
        }
    }

    async fn communication_control(
        &mut self,
        ctrl_type: CommunicationCtrlType,
        comm_type: CommunicationType,
        node_id: Option<request::NodeId>,
        suppress_positive: bool,
        addr_type: AddressType,
    ) -> CanResult<(), Self::Error> {
        let service = Service::CommunicationCtrl;
        let mut sub_func = ctrl_type.into();
        if suppress_positive {
            sub_func |= SUPPRESS_POSITIVE;
        }
        let data = request::CommunicationCtrl::new(ctrl_type, comm_type, node_id)
            .map_err(DoCanError::ISO14229Error)?;
        let cfg = self.context.get_did_cfg().await;
        let req = Request::new(service, Some(sub_func), data.into(), &cfg)
            .map_err(DoCanError::ISO14229Error)?;

        let resp = self
            .suppress_positive_sr(addr_type, req, suppress_positive, &cfg)
            .await?;

        if let Some(response) = resp {
            Self::sub_func_check(&response, ctrl_type.into(), service)?;
        }

        Ok(())
    }

    #[cfg(feature = "std2020")]
    async fn authentication(
        &mut self,
        auth_task: AuthenticationTask,
        data: request::Authentication,
    ) -> CanResult<response::Authentication, Self::Error> {
        let service = Service::Authentication;
        let cfg = self.context.get_did_cfg().await;
        let request = Request::new(service, Some(auth_task.into()), data.into(), &cfg)
            .map_err(DoCanError::ISO14229Error)?;

        let response = self
            .send_and_response(AddressType::Physical, request, &cfg)
            .await?;
        Self::sub_func_check(&response, auth_task.into(), service)?;

        response
            .data::<response::Authentication>(&cfg)
            .map_err(DoCanError::ISO14229Error)
    }

    async fn tester_present(
        &mut self,
        r#type: TesterPresentType,
        suppress_positive: bool,
        addr_type: AddressType,
    ) -> CanResult<(), Self::Error> {
        let cfg = self.context.get_did_cfg().await;
        let (service, request) =
            Self::tester_present_request(r#type, suppress_positive, &cfg).await?;

        let response = self
            .suppress_positive_sr(addr_type, request, suppress_positive, &cfg)
            .await?;

        if let Some(response) = response {
            Self::sub_func_check(&response, r#type.into(), service)?;
        }

        Ok(())
    }

    #[cfg(any(feature = "std2006", feature = "std2013"))]
    async fn access_timing_parameter(
        &mut self,
        r#type: request::TimingParameterAccessType,
        parameter: Vec<u8>,
        suppress_positive: bool,
    ) -> CanResult<Option<response::AccessTimingParameter>, Self::Error> {
        let service = Service::AccessTimingParam;
        let mut sub_func = r#type.into();
        if suppress_positive {
            sub_func |= SUPPRESS_POSITIVE;
        }
        let cfg = self.context.get_did_cfg().await;
        let request = Request::new(service, Some(sub_func), parameter, &cfg)
            .map_err(DoCanError::ISO14229Error)?;

        let response = self
            .suppress_positive_sr(AddressType::Physical, request, suppress_positive, &cfg)
            .await?;

        match response {
            Some(v) => {
                Self::sub_func_check(&v, r#type.into(), service)?;
                Ok(Some(v.data(&cfg).map_err(DoCanError::ISO14229Error)?))
            }
            None => Ok(None),
        }
    }

    async fn secured_data_transmit(
        &mut self,
        apar: AdministrativeParameter,
        signature: SignatureEncryptionCalculation,
        anti_replay_cnt: u16,
        service: u8,
        service_data: Vec<u8>,
        signature_data: Vec<u8>,
    ) -> CanResult<response::SecuredDataTrans, Self::Error> {
        let data = request::SecuredDataTrans::new(
            apar,
            signature,
            anti_replay_cnt,
            service,
            service_data,
            signature_data,
        )
        .map_err(DoCanError::ISO14229Error)?;
        let cfg = self.context.get_did_cfg().await;
        let request = Request::new(Service::SecuredDataTrans, None, data.into(), &cfg)
            .map_err(DoCanError::ISO14229Error)?;

        let response = self
            .send_and_response(AddressType::Physical, request, &cfg)
            .await?;

        response
            .data::<response::SecuredDataTrans>(&cfg)
            .map_err(DoCanError::ISO14229Error)
    }

    async fn control_dtc_setting(
        &mut self,
        r#type: DTCSettingType,
        parameter: Vec<u8>,
        suppress_positive: bool,
    ) -> CanResult<(), Self::Error> {
        let service = Service::CtrlDTCSetting;
        let mut sub_func = r#type.into();
        if suppress_positive {
            sub_func |= SUPPRESS_POSITIVE;
        }
        let cfg = self.context.get_did_cfg().await;
        let request = Request::new(service, Some(sub_func), parameter, &cfg)
            .map_err(DoCanError::ISO14229Error)?;

        let response = self
            .suppress_positive_sr(AddressType::Physical, request, suppress_positive, &cfg)
            .await?;

        if let Some(response) = response {
            Self::sub_func_check(&response, r#type.into(), service)?;
        }

        Ok(())
    }

    async fn response_on_event(&mut self) -> CanResult<(), Self::Error> {
        Err(DoCanError::NotImplement(Service::ResponseOnEvent))
    }

    async fn link_control(
        &mut self,
        r#type: LinkCtrlType,
        data: request::LinkCtrl,
        suppress_positive: bool,
    ) -> CanResult<(), Self::Error> {
        let service = Service::LinkCtrl;
        let mut sub_func = r#type.into();
        if suppress_positive {
            sub_func |= SUPPRESS_POSITIVE;
        }
        let cfg = self.context.get_did_cfg().await;
        let request = Request::new(service, Some(sub_func), data.into(), &cfg)
            .map_err(DoCanError::ISO14229Error)?;

        let response = self
            .suppress_positive_sr(AddressType::Physical, request, suppress_positive, &cfg)
            .await?;

        if let Some(response) = response {
            Self::sub_func_check(&response, r#type.into(), service)?;
        }

        Ok(())
    }

    async fn read_data_by_identifier(
        &mut self,
        did: DataIdentifier,
        others: Vec<DataIdentifier>,
    ) -> CanResult<response::ReadDID, Self::Error> {
        let data = request::ReadDID::new(did, others);
        let cfg = self.context.get_did_cfg().await;
        let request = Request::new(Service::ReadDID, None, data.into(), &cfg)
            .map_err(DoCanError::ISO14229Error)?;

        let response = self
            .send_and_response(AddressType::Physical, request, &cfg)
            .await?;

        response
            .data::<response::ReadDID>(&cfg)
            .map_err(DoCanError::ISO14229Error)
    }

    async fn read_memory_by_address(
        &mut self,
        mem_loc: MemoryLocation,
    ) -> CanResult<Vec<u8>, Self::Error> {
        let data = request::ReadMemByAddr(mem_loc);
        let cfg = self.context.get_did_cfg().await;
        let request = Request::new(Service::ReadMemByAddr, None, data.into(), &cfg)
            .map_err(DoCanError::ISO14229Error)?;

        let response = self
            .send_and_response(AddressType::Physical, request, &cfg)
            .await?;

        Ok(response.raw_data().to_vec())
    }

    async fn read_scaling_data_by_identifier(
        &mut self,
        did: DataIdentifier,
    ) -> CanResult<response::ReadScalingDID, Self::Error> {
        let data = request::ReadScalingDID(did);
        let cfg = self.context.get_did_cfg().await;
        let request = Request::new(Service::ReadScalingDID, None, data.into(), &cfg)
            .map_err(DoCanError::ISO14229Error)?;

        let response = self
            .send_and_response(AddressType::Physical, request, &cfg)
            .await?;

        response
            .data::<response::ReadScalingDID>(&cfg)
            .map_err(DoCanError::ISO14229Error)
    }

    async fn read_data_by_period_identifier(
        &mut self,
        mode: request::TransmissionMode,
        did: Vec<u8>,
    ) -> CanResult<response::ReadDataByPeriodId, Self::Error> {
        let data =
            request::ReadDataByPeriodId::new(mode, did).map_err(DoCanError::ISO14229Error)?;
        let cfg = self.context.get_did_cfg().await;
        let request = Request::new(Service::ReadDataByPeriodId, None, data.into(), &cfg)
            .map_err(DoCanError::ISO14229Error)?;

        let response = self
            .send_and_response(AddressType::Physical, request, &cfg)
            .await?;

        response
            .data::<response::ReadDataByPeriodId>(&cfg)
            .map_err(DoCanError::ISO14229Error)
    }

    async fn dynamically_define_data_by_identifier(
        &mut self,
        r#type: DefinitionType,
        data: request::DynamicallyDefineDID,
        suppress_positive: bool,
    ) -> CanResult<Option<response::DynamicallyDefineDID>, Self::Error> {
        let service = Service::DynamicalDefineDID;
        let mut sub_func = r#type.into();
        if suppress_positive {
            sub_func |= SUPPRESS_POSITIVE;
        }
        let cfg = self.context.get_did_cfg().await;
        let request = Request::new(service, Some(sub_func), data.into(), &cfg)
            .map_err(DoCanError::ISO14229Error)?;

        let response = self
            .suppress_positive_sr(AddressType::Physical, request, suppress_positive, &cfg)
            .await?;

        match response {
            Some(v) => {
                Self::sub_func_check(&v, r#type.into(), service)?;
                Ok(Some(v.data(&cfg).map_err(DoCanError::ISO14229Error)?))
            }
            None => Ok(None),
        }
    }

    async fn write_data_by_identifier(
        &mut self,
        did: DataIdentifier,
        data: Vec<u8>,
    ) -> CanResult<(), Self::Error> {
        let data = request::WriteDID(DIDData { did, data });
        let cfg = self.context.get_did_cfg().await;
        let request = Request::new(Service::WriteDID, None, data.into(), &cfg)
            .map_err(DoCanError::ISO14229Error)?;

        let _ = self
            .send_and_response(AddressType::Physical, request, &cfg)
            .await?;

        Ok(())
    }

    async fn write_memory_by_address(
        &mut self,
        alfi: AddressAndLengthFormatIdentifier,
        mem_addr: u128,
        mem_size: u128,
        record: Vec<u8>,
    ) -> CanResult<response::WriteMemByAddr, Self::Error> {
        let data = request::WriteMemByAddr::new(alfi, mem_addr, mem_size, record)
            .map_err(DoCanError::ISO14229Error)?;
        let cfg = self.context.get_did_cfg().await;
        let request = Request::new(Service::WriteMemByAddr, None, data.into(), &cfg)
            .map_err(DoCanError::ISO14229Error)?;

        let response = self
            .send_and_response(AddressType::Physical, request, &cfg)
            .await?;

        response
            .data::<response::WriteMemByAddr>(&cfg)
            .map_err(DoCanError::ISO14229Error)
    }

    async fn clear_dtc_info(
        &mut self,
        group: U24,
        mem_sel: Option<u8>,
        addr_type: AddressType,
    ) -> CanResult<(), Self::Error> {
        #[cfg(any(feature = "std2020"))]
        let data = request::ClearDiagnosticInfo::new(group, mem_sel);
        #[cfg(any(feature = "std2006", feature = "std2013"))]
        let data = request::ClearDiagnosticInfo::new(group);
        let cfg = self.context.get_did_cfg().await;
        let request = Request::new(Service::ClearDiagnosticInfo, None, data.into(), &cfg)
            .map_err(DoCanError::ISO14229Error)?;

        let _ = self.send_and_response(addr_type, request, &cfg).await?;

        Ok(())
    }

    async fn read_dtc_info(
        &mut self,
        r#type: DTCReportType,
        data: request::DTCInfo,
    ) -> CanResult<response::DTCInfo, Self::Error> {
        let service = Service::ReadDTCInfo;
        let cfg = self.context.get_did_cfg().await;
        let request = Request::new(service, Some(r#type.into()), data.into(), &cfg)
            .map_err(DoCanError::ISO14229Error)?;

        let response = self
            .send_and_response(AddressType::Physical, request, &cfg)
            .await?;
        Self::sub_func_check(&response, r#type.into(), service)?;

        response
            .data::<response::DTCInfo>(&cfg)
            .map_err(DoCanError::ISO14229Error)
    }

    async fn io_control(
        &mut self,
        did: DataIdentifier,
        param: IOCtrlParameter,
        state: Vec<u8>,
        mask: Vec<u8>,
    ) -> CanResult<response::IOCtrl, Self::Error> {
        let cfg = self.context.get_did_cfg().await;
        let data = request::IOCtrl::new(did, param, state, mask, &cfg)
            .map_err(DoCanError::ISO14229Error)?;
        let request = Request::new(Service::IOCtrl, None, data.into(), &cfg)
            .map_err(DoCanError::ISO14229Error)?;

        let response = self
            .send_and_response(AddressType::Physical, request, &cfg)
            .await?;

        response
            .data::<response::IOCtrl>(&cfg)
            .map_err(DoCanError::ISO14229Error)
    }

    async fn routine_control(
        &mut self,
        r#type: RoutineCtrlType,
        routine_id: u16,
        option_record: Vec<u8>,
    ) -> CanResult<response::RoutineCtrl, Self::Error> {
        let service = Service::RoutineCtrl;
        let data = request::RoutineCtrl {
            routine_id: RoutineId(routine_id),
            option_record,
        };
        let cfg = self.context.get_did_cfg().await;
        let request = Request::new(service, Some(r#type.into()), data.into(), &cfg)
            .map_err(DoCanError::ISO14229Error)?;

        let response = self
            .send_and_response(AddressType::Physical, request, &cfg)
            .await?;
        Self::sub_func_check(&response, r#type.into(), service)?;

        response
            .data::<response::RoutineCtrl>(&cfg)
            .map_err(DoCanError::ISO14229Error)
    }

    async fn request_download(
        &mut self,
        alfi: AddressAndLengthFormatIdentifier,
        mem_addr: u128,
        mem_size: u128,
        dfi: Option<DataFormatIdentifier>,
    ) -> CanResult<response::RequestDownload, Self::Error> {
        let data = request::RequestDownload {
            dfi: dfi.unwrap_or_default(),
            mem_loc: MemoryLocation::new(alfi, mem_addr, mem_size)
                .map_err(DoCanError::ISO14229Error)?,
        };
        let cfg = self.context.get_did_cfg().await;
        let request = Request::new(Service::RequestDownload, None, data.into(), &cfg)
            .map_err(DoCanError::ISO14229Error)?;

        let response = self
            .send_and_response(AddressType::Physical, request, &cfg)
            .await?;

        response
            .data::<response::RequestDownload>(&cfg)
            .map_err(DoCanError::ISO14229Error)
    }

    async fn request_upload(
        &mut self,
        alfi: AddressAndLengthFormatIdentifier,
        mem_addr: u128,
        mem_size: u128,
        dfi: Option<DataFormatIdentifier>,
    ) -> CanResult<response::RequestUpload, Self::Error> {
        let data = request::RequestUpload {
            dfi: dfi.unwrap_or_default(),
            mem_loc: MemoryLocation::new(alfi, mem_addr, mem_size)
                .map_err(DoCanError::ISO14229Error)?,
        };
        let cfg = self.context.get_did_cfg().await;
        let request = Request::new(Service::RequestDownload, None, data.into(), &cfg)
            .map_err(DoCanError::ISO14229Error)?;

        let response = self
            .send_and_response(AddressType::Physical, request, &cfg)
            .await?;

        response
            .data::<response::RequestUpload>(&cfg)
            .map_err(DoCanError::ISO14229Error)
    }

    async fn transfer_data(
        &mut self,
        sequence: u8,
        data: Vec<u8>,
    ) -> CanResult<response::TransferData, Self::Error> {
        let data = response::TransferData { sequence, data };
        let cfg = self.context.get_did_cfg().await;
        let request = Request::new(Service::TransferData, None, data.into(), &cfg)
            .map_err(DoCanError::ISO14229Error)?;

        let response = self
            .send_and_response(AddressType::Physical, request, &cfg)
            .await?;

        let data = response
            .data::<response::TransferData>(&cfg)
            .map_err(DoCanError::ISO14229Error)?;

        if data.sequence != sequence {
            return Err(DoCanError::UnexpectedTransferSequence {
                expect: sequence,
                actual: data.sequence,
            });
        }

        Ok(data)
    }

    async fn request_transfer_exit(
        &mut self,
        parameter: Vec<u8>,
    ) -> CanResult<Vec<u8>, Self::Error> {
        let cfg = self.context.get_did_cfg().await;
        let request = Request::new(Service::RequestTransferExit, None, parameter, &cfg)
            .map_err(DoCanError::ISO14229Error)?;

        let response = self
            .send_and_response(AddressType::Physical, request, &cfg)
            .await?;

        Ok(response.raw_data().to_vec())
    }

    #[cfg(any(feature = "std2013", feature = "std2020"))]
    async fn request_file_transfer(
        &mut self,
        operation: ModeOfOperation,
        data: request::RequestFileTransfer,
    ) -> CanResult<response::RequestFileTransfer, Self::Error> {
        let service = Service::RequestFileTransfer;
        let sub_func = operation.into();
        let cfg = self.context.get_did_cfg().await;
        let request = Request::new(service, Some(sub_func), data.into(), &cfg)
            .map_err(DoCanError::ISO14229Error)?;

        let response = self
            .send_and_response(AddressType::Physical, request, &cfg)
            .await?;
        Self::sub_func_check(&response, operation.into(), service)?;

        response
            .data::<response::RequestFileTransfer>(&cfg)
            .map_err(DoCanError::ISO14229Error)
    }
}

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

    async fn update_address(
        &mut self,
        channel: Self::Channel,
        address: Address,
    ) -> CanResult<(), Self::Error> {
        self.with_context(channel, |_, iso_tp, _| async move {
            iso_tp.update_address(address).await;

            Ok(())
        })
        .await
    }

    async fn update_security_algo(
        &mut self,
        channel: Self::Channel,
        algo: SecurityAlgo,
    ) -> CanResult<(), Self::Error> {
        self.context_util(channel, |ctx| async move {
            let mut guard = ctx.security_algo.lock().await;
            *guard = Some(algo);

            Ok(())
        })
        .await
    }

    async fn add_data_identifier(
        &mut self,
        channel: Self::Channel,
        did: DataIdentifier,
        length: usize,
    ) -> CanResult<(), Self::Error> {
        self.context_util(channel, |ctx| async move {
            ctx.did.lock().await.insert(did, length);

            Ok(())
        })
        .await
    }

    async fn remove_data_identifier(
        &mut self,
        channel: Self::Channel,
        did: DataIdentifier,
    ) -> CanResult<(), Self::Error> {
        self.context_util(channel, |ctx| async move {
            ctx.did.lock().await.remove(&did);

            Ok(())
        })
        .await
    }

    async fn session_ctrl(
        &mut self,
        channel: Self::Channel,
        r#type: SessionType,
        suppress_positive: bool,
        addr_type: AddressType,
    ) -> CanResult<(), Self::Error> {
        self.with_context(channel, |listener, iso_tp, did| async move {
            let service = Service::SessionCtrl;
            let mut sub_func: u8 = r#type.into();
            if suppress_positive {
                sub_func |= SUPPRESS_POSITIVE;
            }
            let request = Request::new(service, Some(sub_func), vec![], &did)
                .map_err(DoCanError::ISO14229Error)?;

            if let Some(response) = Self::suppress_positive_sr(
                &listener,
                &iso_tp,
                &did,
                addr_type,
                request,
                suppress_positive,
            )
            .await?
            {
                Self::sub_func_check(&response, r#type.into(), service)?;

                let timing = response
                    .data::<response::SessionCtrl>(&did)
                    .map_err(DoCanError::ISO14229Error)?
                    .0;
                listener
                    .update_p2_ctx(timing.p2, timing.p2_star as u32)
                    .await;
            }

            Ok(())
        })
        .await
    }

    async fn ecu_reset(
        &mut self,
        channel: Self::Channel,
        r#type: ECUResetType,
        suppress_positive: bool,
        addr_type: AddressType,
    ) -> CanResult<(), Self::Error> {
        self.with_context(channel, |listener, iso_tp, did| async move {
            let service = Service::ECUReset;
            let mut sub_func: u8 = r#type.into();
            if suppress_positive {
                sub_func |= SUPPRESS_POSITIVE;
            }
            let request = Request::new(service, Some(sub_func), vec![], &did)
                .map_err(DoCanError::ISO14229Error)?;

            if let Some(response) = Self::suppress_positive_sr(
                &listener,
                &iso_tp,
                &did,
                addr_type,
                request,
                suppress_positive,
            )
            .await?
            {
                Self::sub_func_check(&response, r#type.into(), service)?;

                let resp = response
                    .data::<response::ECUReset>(&did)
                    .map_err(DoCanError::ISO14229Error)?;
                if let Some(seconds) = resp.second {
                    sleep(Duration::from_secs(seconds as u64)).await;
                }
            }

            Ok(())
        })
        .await
    }

    async fn security_access(
        &mut self,
        channel: Self::Channel,
        level: u8,
        params: Vec<u8>,
    ) -> CanResult<Vec<u8>, Self::Error> {
        self.with_context(channel, |listener, iso_tp, did| {
            let params = params.clone();
            async move {
                let service = Service::SecurityAccess;
                let request = Request::new(service, Some(level), params, &did)
                    .map_err(DoCanError::ISO14229Error)?;

                let response = Self::send_and_response(
                    &listener,
                    &iso_tp,
                    &did,
                    AddressType::Physical,
                    request,
                )
                .await?;

                Self::sub_func_check(&response, level, service)?;

                Ok(response.raw_data().to_vec())
            }
        })
        .await
    }

    async fn unlock_security_access(
        &mut self,
        channel: Self::Channel,
        level: u8,
        params: Vec<u8>,
        salt: Vec<u8>,
    ) -> CanResult<(), Self::Error> {
        self.with_security_algo_ctx(channel, |listener, iso_tp, did, algo| {
            let params = params.clone();
            let salt = salt.clone();
            async move {
                let service = Service::SecurityAccess;
                let req = Request::new(service, Some(level), params, &did)
                    .map_err(DoCanError::ISO14229Error)?;

                let resp =
                    Self::send_and_response(&listener, &iso_tp, &did, AddressType::Physical, req)
                        .await?;
                Self::sub_func_check(&resp, level, service)?;

                let seed = resp.raw_data().to_vec();
                match algo(level, seed, salt)? {
                    Some(data) => {
                        let request = Request::new(service, Some(level + 1), data, &did)
                            .map_err(DoCanError::ISO14229Error)?;
                        let response = Self::send_and_response(
                            &listener,
                            &iso_tp,
                            &did,
                            AddressType::Physical,
                            request,
                        )
                        .await?;

                        Self::sub_func_check(&response, level + 1, service)
                    }
                    None => Ok(()),
                }
            }
        })
        .await
    }

    async fn communication_control(
        &mut self,
        channel: Self::Channel,
        ctrl_type: CommunicationCtrlType,
        comm_type: CommunicationType,
        node_id: Option<request::NodeId>,
        suppress_positive: bool,
        addr_type: AddressType,
    ) -> CanResult<(), Self::Error> {
        self.with_context(channel, |listener, iso_tp, did| async move {
            let service = Service::CommunicationCtrl;
            let mut sub_func = ctrl_type.into();
            if suppress_positive {
                sub_func |= SUPPRESS_POSITIVE;
            }
            let data = request::CommunicationCtrl::new(ctrl_type, comm_type, node_id)
                .map_err(DoCanError::ISO14229Error)?;
            let req = Request::new(service, Some(sub_func), data.into(), &did)
                .map_err(DoCanError::ISO14229Error)?;

            let resp = Self::suppress_positive_sr(
                &listener,
                &iso_tp,
                &did,
                addr_type,
                req,
                suppress_positive,
            )
            .await?;

            if let Some(response) = resp {
                Self::sub_func_check(&response, ctrl_type.into(), service)?;
            }

            Ok(())
        })
        .await
    }

    #[cfg(feature = "std2020")]
    async fn authentication(
        &mut self,
        channel: Self::Channel,
        auth_task: AuthenticationTask,
        data: request::Authentication,
    ) -> CanResult<response::Authentication, Self::Error> {
        self.with_context(channel, |listener, iso_tp, did| {
            let data = data.clone();
            async move {
                let service = Service::Authentication;
                let request = Request::new(service, Some(auth_task.into()), data.into(), &did)
                    .map_err(DoCanError::ISO14229Error)?;

                let response = Self::send_and_response(
                    &listener,
                    &iso_tp,
                    &did,
                    AddressType::Physical,
                    request,
                )
                .await?;
                Self::sub_func_check(&response, auth_task.into(), service)?;

                response
                    .data::<response::Authentication>(&did)
                    .map_err(DoCanError::ISO14229Error)
            }
        })
        .await
    }

    async fn tester_present(
        &mut self,
        channel: Self::Channel,
        r#type: TesterPresentType,
        suppress_positive: bool,
        addr_type: AddressType,
    ) -> CanResult<(), Self::Error> {
        self.with_context(channel, |listener, iso_tp, did| async move {
            let (service, request) =
                Self::tester_present_request(&did, r#type, suppress_positive).await?;

            let response = Self::suppress_positive_sr(
                &listener,
                &iso_tp,
                &did,
                addr_type,
                request,
                suppress_positive,
            )
            .await?;

            if let Some(response) = response {
                Self::sub_func_check(&response, r#type.into(), service)?;
            }

            Ok(())
        })
        .await
    }

    #[cfg(any(feature = "std2006", feature = "std2013"))]
    async fn access_timing_parameter(
        &mut self,
        channel: Self::Channel,
        r#type: request::TimingParameterAccessType,
        parameter: Vec<u8>,
        suppress_positive: bool,
    ) -> CanResult<Option<response::AccessTimingParameter>, Self::Error> {
        self.with_context(channel, |listener, iso_tp, did| async move {
            let service = Service::AccessTimingParam;
            let mut sub_func = r#type.into();
            if suppress_positive {
                sub_func |= SUPPRESS_POSITIVE;
            }
            let request = Request::new(service, Some(sub_func), parameter, &did)
                .map_err(DoCanError::ISO14229Error)?;

            let response = Self::suppress_positive_sr(
                &listener,
                &iso_tp,
                &did,
                AddressType::Physical,
                request,
                suppress_positive,
            )
            .await?;

            match response {
                Some(v) => {
                    Self::sub_func_check(&v, r#type.into(), service)?;
                    Ok(Some(v.data().map_err(DoCanError::ISO14229Error)?))
                }
                None => Ok(None),
            }
        })
        .await
    }

    async fn secured_data_transmit(
        &mut self,
        channel: Self::Channel,
        apar: AdministrativeParameter,
        signature: SignatureEncryptionCalculation,
        anti_replay_cnt: u16,
        service: u8,
        service_data: Vec<u8>,
        signature_data: Vec<u8>,
    ) -> CanResult<response::SecuredDataTrans, Self::Error> {
        self.with_context(channel, |listener, iso_tp, did| {
            let service_data = service_data.clone();
            let signature_data = signature_data.clone();
            async move {
                let data = request::SecuredDataTrans::new(
                    apar,
                    signature,
                    anti_replay_cnt,
                    service,
                    service_data,
                    signature_data,
                )
                .map_err(DoCanError::ISO14229Error)?;
                let request = Request::new(Service::SecuredDataTrans, None, data.into(), &did)
                    .map_err(DoCanError::ISO14229Error)?;

                let response = Self::send_and_response(
                    &listener,
                    &iso_tp,
                    &did,
                    AddressType::Physical,
                    request,
                )
                .await?;

                response
                    .data::<response::SecuredDataTrans>(&did)
                    .map_err(DoCanError::ISO14229Error)
            }
        })
        .await
    }

    async fn control_dtc_setting(
        &mut self,
        channel: Self::Channel,
        r#type: DTCSettingType,
        parameter: Vec<u8>,
        suppress_positive: bool,
    ) -> CanResult<(), Self::Error> {
        self.with_context(channel, |listener, iso_tp, did| {
            let parameter = parameter.clone();
            async move {
                let service = Service::CtrlDTCSetting;
                let mut sub_func = r#type.into();
                if suppress_positive {
                    sub_func |= SUPPRESS_POSITIVE;
                }
                let request = Request::new(service, Some(sub_func), parameter, &did)
                    .map_err(DoCanError::ISO14229Error)?;

                let response = Self::suppress_positive_sr(
                    &listener,
                    &iso_tp,
                    &did,
                    AddressType::Physical,
                    request,
                    suppress_positive,
                )
                .await?;

                if let Some(response) = response {
                    Self::sub_func_check(&response, r#type.into(), service)?;
                }

                Ok(())
            }
        })
        .await
    }

    async fn response_on_event(&mut self, channel: Self::Channel) -> CanResult<(), Self::Error> {
        self.with_context(channel, |_, _, _| async move {
            Err(DoCanError::NotImplement(Service::ResponseOnEvent))
        })
        .await
    }

    async fn link_control(
        &mut self,
        channel: Self::Channel,
        r#type: LinkCtrlType,
        data: request::LinkCtrl,
        suppress_positive: bool,
    ) -> CanResult<(), Self::Error> {
        self.with_context(channel, |listener, iso_tp, did| {
            let data = data.clone();
            async move {
                let service = Service::LinkCtrl;
                let mut sub_func = r#type.into();
                if suppress_positive {
                    sub_func |= SUPPRESS_POSITIVE;
                }
                let request = Request::new(service, Some(sub_func), data.into(), &did)
                    .map_err(DoCanError::ISO14229Error)?;

                let response = Self::suppress_positive_sr(
                    &listener,
                    &iso_tp,
                    &did,
                    AddressType::Physical,
                    request,
                    suppress_positive,
                )
                .await?;

                if let Some(response) = response {
                    Self::sub_func_check(&response, r#type.into(), service)?;
                }

                Ok(())
            }
        })
        .await
    }

    async fn read_data_by_identifier(
        &mut self,
        channel: Self::Channel,
        did: DataIdentifier,
        others: Vec<DataIdentifier>,
    ) -> CanResult<response::ReadDID, Self::Error> {
        self.with_context(channel, |listener, iso_tp, cfg| {
            let others = others.clone();
            async move {
                let data = request::ReadDID::new(did, others);
                let request = Request::new(Service::ReadDID, None, data.into(), &cfg)
                    .map_err(DoCanError::ISO14229Error)?;

                let response = Self::send_and_response(
                    &listener,
                    &iso_tp,
                    &cfg,
                    AddressType::Physical,
                    request,
                )
                .await?;

                response
                    .data::<response::ReadDID>(&cfg)
                    .map_err(DoCanError::ISO14229Error)
            }
        })
        .await
    }

    async fn read_memory_by_address(
        &mut self,
        channel: Self::Channel,
        mem_loc: MemoryLocation,
    ) -> CanResult<Vec<u8>, Self::Error> {
        self.with_context(channel, |listener, iso_tp, did| async move {
            let data = request::ReadMemByAddr(mem_loc);
            let request = Request::new(Service::ReadMemByAddr, None, data.into(), &did)
                .map_err(DoCanError::ISO14229Error)?;

            let response =
                Self::send_and_response(&listener, &iso_tp, &did, AddressType::Physical, request)
                    .await?;

            Ok(response.raw_data().to_vec())
        })
        .await
    }

    async fn read_scaling_data_by_identifier(
        &mut self,
        channel: Self::Channel,
        did: DataIdentifier,
    ) -> CanResult<response::ReadScalingDID, Self::Error> {
        self.with_context(channel, |listener, iso_tp, cfg| async move {
            let data = request::ReadScalingDID(did);
            let request = Request::new(Service::ReadScalingDID, None, data.into(), &cfg)
                .map_err(DoCanError::ISO14229Error)?;

            let response =
                Self::send_and_response(&listener, &iso_tp, &cfg, AddressType::Physical, request)
                    .await?;

            response
                .data::<response::ReadScalingDID>(&cfg)
                .map_err(DoCanError::ISO14229Error)
        })
        .await
    }

    async fn read_data_by_period_identifier(
        &mut self,
        channel: Self::Channel,
        mode: request::TransmissionMode,
        did: Vec<u8>,
    ) -> CanResult<response::ReadDataByPeriodId, Self::Error> {
        self.with_context(channel, |listener, iso_tp, cfg| {
            let did = did.clone();
            async move {
                let data = request::ReadDataByPeriodId::new(mode, did)
                    .map_err(DoCanError::ISO14229Error)?;
                let request = Request::new(Service::ReadDataByPeriodId, None, data.into(), &cfg)
                    .map_err(DoCanError::ISO14229Error)?;

                let response = Self::send_and_response(
                    &listener,
                    &iso_tp,
                    &cfg,
                    AddressType::Physical,
                    request,
                )
                .await?;

                response
                    .data::<response::ReadDataByPeriodId>(&cfg)
                    .map_err(DoCanError::ISO14229Error)
            }
        })
        .await
    }

    async fn dynamically_define_data_by_identifier(
        &mut self,
        channel: Self::Channel,
        r#type: DefinitionType,
        data: request::DynamicallyDefineDID,
        suppress_positive: bool,
    ) -> CanResult<Option<response::DynamicallyDefineDID>, Self::Error> {
        self.with_context(channel, |listener, iso_tp, did| {
            let data = data.clone();
            async move {
                let service = Service::DynamicalDefineDID;
                let mut sub_func = r#type.into();
                if suppress_positive {
                    sub_func |= SUPPRESS_POSITIVE;
                }
                let request = Request::new(service, Some(sub_func), data.into(), &did)
                    .map_err(DoCanError::ISO14229Error)?;

                let response = Self::suppress_positive_sr(
                    &listener,
                    &iso_tp,
                    &did,
                    AddressType::Physical,
                    request,
                    suppress_positive,
                )
                .await?;

                match response {
                    Some(v) => {
                        Self::sub_func_check(&v, r#type.into(), service)?;
                        Ok(Some(v.data(&did).map_err(DoCanError::ISO14229Error)?))
                    }
                    None => Ok(None),
                }
            }
        })
        .await
    }

    async fn write_data_by_identifier(
        &mut self,
        channel: Self::Channel,
        did: DataIdentifier,
        data: Vec<u8>,
    ) -> CanResult<(), Self::Error> {
        self.with_context(channel, |listener, iso_tp, cfg| {
            let data = data.clone();
            async move {
                let data = request::WriteDID(DIDData { did, data });
                let request = Request::new(Service::WriteDID, None, data.into(), &cfg)
                    .map_err(DoCanError::ISO14229Error)?;

                let _ = Self::send_and_response(
                    &listener,
                    &iso_tp,
                    &cfg,
                    AddressType::Physical,
                    request,
                )
                .await?;

                Ok(())
            }
        })
        .await
    }

    async fn write_memory_by_address(
        &mut self,
        channel: Self::Channel,
        alfi: AddressAndLengthFormatIdentifier,
        mem_addr: u128,
        mem_size: u128,
        record: Vec<u8>,
    ) -> CanResult<response::WriteMemByAddr, Self::Error> {
        self.with_context(channel, |listener, iso_tp, did| {
            let record = record.clone();
            async move {
                let data = request::WriteMemByAddr::new(alfi, mem_addr, mem_size, record)
                    .map_err(DoCanError::ISO14229Error)?;
                let request = Request::new(Service::WriteMemByAddr, None, data.into(), &did)
                    .map_err(DoCanError::ISO14229Error)?;

                let response = Self::send_and_response(
                    &listener,
                    &iso_tp,
                    &did,
                    AddressType::Physical,
                    request,
                )
                .await?;

                response
                    .data::<response::WriteMemByAddr>(&did)
                    .map_err(DoCanError::ISO14229Error)
            }
        })
        .await
    }

    async fn clear_dtc_info(
        &mut self,
        channel: Self::Channel,
        group: U24,
        mem_sel: Option<u8>,
        addr_type: AddressType,
    ) -> CanResult<(), Self::Error> {
        self.with_context(channel, |listener, iso_tp, did| async move {
            let group = group.clone();
            async move {
                #[cfg(any(feature = "std2020"))]
                let data = request::ClearDiagnosticInfo::new(group, mem_sel);
                #[cfg(any(feature = "std2006", feature = "std2013"))]
                let data = request::ClearDiagnosticInfo::new(group);
                let request = Request::new(Service::ClearDiagnosticInfo, None, data.into(), &did)
                    .map_err(DoCanError::ISO14229Error)?;

                let _ =
                    Self::send_and_response(&listener, &iso_tp, &did, addr_type, request).await?;

                Ok(())
            }
            .await
        })
        .await
    }

    async fn read_dtc_info(
        &mut self,
        channel: Self::Channel,
        r#type: DTCReportType,
        data: request::DTCInfo,
    ) -> CanResult<response::DTCInfo, Self::Error> {
        self.with_context(channel, |listener, iso_tp, did| async move {
            let data = data.clone();
            async move {
                let service = Service::ReadDTCInfo;
                let request = Request::new(service, Some(r#type.into()), data.into(), &did)
                    .map_err(DoCanError::ISO14229Error)?;

                let response = Self::send_and_response(
                    &listener,
                    &iso_tp,
                    &did,
                    AddressType::Physical,
                    request,
                )
                .await?;
                Self::sub_func_check(&response, r#type.into(), service)?;

                response
                    .data::<response::DTCInfo>(&did)
                    .map_err(DoCanError::ISO14229Error)
            }
            .await
        })
        .await
    }

    async fn io_control(
        &mut self,
        channel: Self::Channel,
        did: DataIdentifier,
        param: IOCtrlParameter,
        state: Vec<u8>,
        mask: Vec<u8>,
    ) -> CanResult<response::IOCtrl, Self::Error> {
        self.with_context(channel, |listener, iso_tp, cfg| {
            let state = state.clone();
            let mask = mask.clone();
            async move {
                let data = request::IOCtrl::new(did, param, state, mask, &cfg)
                    .map_err(DoCanError::ISO14229Error)?;
                let request = Request::new(Service::IOCtrl, None, data.into(), &cfg)
                    .map_err(DoCanError::ISO14229Error)?;

                let response = Self::send_and_response(
                    &listener,
                    &iso_tp,
                    &cfg,
                    AddressType::Physical,
                    request,
                )
                .await?;

                response
                    .data::<response::IOCtrl>(&cfg)
                    .map_err(DoCanError::ISO14229Error)
            }
        })
        .await
    }

    async fn routine_control(
        &mut self,
        channel: Self::Channel,
        r#type: RoutineCtrlType,
        routine_id: u16,
        option_record: Vec<u8>,
    ) -> CanResult<response::RoutineCtrl, Self::Error> {
        self.with_context(channel, |listener, iso_tp, did| {
            let option_record = option_record.clone();
            async move {
                let service = Service::RoutineCtrl;
                let data = request::RoutineCtrl {
                    routine_id: RoutineId(routine_id),
                    option_record,
                };
                let request = Request::new(service, Some(r#type.into()), data.into(), &did)
                    .map_err(DoCanError::ISO14229Error)?;

                let response = Self::send_and_response(
                    &listener,
                    &iso_tp,
                    &did,
                    AddressType::Physical,
                    request,
                )
                .await?;
                Self::sub_func_check(&response, r#type.into(), service)?;

                response
                    .data::<response::RoutineCtrl>(&did)
                    .map_err(DoCanError::ISO14229Error)
            }
        })
        .await
    }

    async fn request_download(
        &mut self,
        channel: Self::Channel,
        alfi: AddressAndLengthFormatIdentifier,
        mem_addr: u128,
        mem_size: u128,
        dfi: Option<DataFormatIdentifier>,
    ) -> CanResult<response::RequestDownload, Self::Error> {
        self.with_context(channel, |listener, iso_tp, did| async move {
            let data = request::RequestDownload {
                dfi: dfi.unwrap_or_default(),
                mem_loc: MemoryLocation::new(alfi, mem_addr, mem_size)
                    .map_err(DoCanError::ISO14229Error)?,
            };
            let request = Request::new(Service::RequestDownload, None, data.into(), &did)
                .map_err(DoCanError::ISO14229Error)?;

            let response =
                Self::send_and_response(&listener, &iso_tp, &did, AddressType::Physical, request)
                    .await?;

            response
                .data::<response::RequestDownload>(&did)
                .map_err(DoCanError::ISO14229Error)
        })
        .await
    }

    async fn request_upload(
        &mut self,
        channel: Self::Channel,
        alfi: AddressAndLengthFormatIdentifier,
        mem_addr: u128,
        mem_size: u128,
        dfi: Option<DataFormatIdentifier>,
    ) -> CanResult<response::RequestUpload, Self::Error> {
        self.with_context(channel, |listener, iso_tp, did| async move {
            let data = request::RequestUpload {
                dfi: dfi.unwrap_or_default(),
                mem_loc: MemoryLocation::new(alfi, mem_addr, mem_size)
                    .map_err(DoCanError::ISO14229Error)?,
            };
            let request = Request::new(Service::RequestDownload, None, data.into(), &did)
                .map_err(DoCanError::ISO14229Error)?;

            let response =
                Self::send_and_response(&listener, &iso_tp, &did, AddressType::Physical, request)
                    .await?;

            response
                .data::<response::RequestUpload>(&did)
                .map_err(DoCanError::ISO14229Error)
        })
        .await
    }

    async fn transfer_data(
        &mut self,
        channel: Self::Channel,
        sequence: u8,
        data: Vec<u8>,
    ) -> CanResult<response::TransferData, Self::Error> {
        self.with_context(channel, |listener, iso_tp, did| {
            let data = data.clone();
            async move {
                let data = response::TransferData { sequence, data };
                let request = Request::new(Service::TransferData, None, data.into(), &did)
                    .map_err(DoCanError::ISO14229Error)?;

                let response = Self::send_and_response(
                    &listener,
                    &iso_tp,
                    &did,
                    AddressType::Physical,
                    request,
                )
                .await?;

                let data = response
                    .data::<response::TransferData>(&did)
                    .map_err(DoCanError::ISO14229Error)?;

                if data.sequence != sequence {
                    return Err(DoCanError::UnexpectedTransferSequence {
                        expect: sequence,
                        actual: data.sequence,
                    });
                }

                Ok(data)
            }
        })
        .await
    }

    async fn request_transfer_exit(
        &mut self,
        channel: Self::Channel,
        parameter: Vec<u8>,
    ) -> CanResult<Vec<u8>, Self::Error> {
        self.with_context(channel, |listener, iso_tp, did| {
            let parameter = parameter.clone();
            async move {
                let request = Request::new(Service::RequestTransferExit, None, parameter, &did)
                    .map_err(DoCanError::ISO14229Error)?;

                let response = Self::send_and_response(
                    &listener,
                    &iso_tp,
                    &did,
                    AddressType::Physical,
                    request,
                )
                .await?;

                Ok(response.raw_data().to_vec())
            }
        })
        .await
    }

    #[cfg(any(feature = "std2013", feature = "std2020"))]
    async fn request_file_transfer(
        &mut self,
        channel: Self::Channel,
        operation: ModeOfOperation,
        data: request::RequestFileTransfer,
    ) -> CanResult<response::RequestFileTransfer, Self::Error> {
        self.with_context(channel, |listener, iso_tp, did| {
            let data = data.clone();
            async move {
                let service = Service::RequestFileTransfer;
                let sub_func = operation.into();
                let request = Request::new(service, Some(sub_func), data.into(), &did)
                    .map_err(DoCanError::ISO14229Error)?;

                let response = Self::send_and_response(
                    &listener,
                    &iso_tp,
                    &did,
                    AddressType::Physical,
                    request,
                )
                .await?;
                Self::sub_func_check(&response, operation.into(), service)?;

                response
                    .data::<response::RequestFileTransfer>(&did)
                    .map_err(DoCanError::ISO14229Error)
            }
        })
        .await
    }
}

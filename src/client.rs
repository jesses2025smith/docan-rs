mod client_impl;
mod context;

pub use client_impl::*;

use crate::SecurityAlgo;
use iso14229_1::{request, response, *};
use iso15765_2::{Address, AddressType};
use rs_can::CanResult;

#[async_trait::async_trait]
pub trait Client {
    type Channel;
    // type Device;
    type Error;

    fn channel(&self) -> Self::Channel;

    async fn update_address(&self, address: Address);
    async fn update_security_algo(&self, algo: SecurityAlgo);
    async fn add_data_identifier(&self, did: DataIdentifier, length: usize);
    async fn remove_data_identifier(&self, did: DataIdentifier);
    // async fn set_address_of_byte_order(
    //     &mut self,
    //     bo: ByteOrder,
    // ) -> CanResult<(), Self::Error>;
    // async fn set_memory_size_of_byte_order(
    //     &mut self,
    //     bo: ByteOrder,
    // ) -> CanResult<(), Self::Error>;
    /** - Diagnostic and communication management functional unit - **/
    async fn session_ctrl(
        &mut self,
        r#type: SessionType,
        suppress_positive: bool,
        addr_type: AddressType,
    ) -> CanResult<(), Self::Error>;
    async fn ecu_reset(
        &mut self,
        r#type: ECUResetType,
        suppress_positive: bool,
        addr_type: AddressType,
    ) -> CanResult<(), Self::Error>;
    async fn security_access(
        &mut self,
        level: u8,
        params: Vec<u8>,
    ) -> CanResult<Vec<u8>, Self::Error>;
    async fn unlock_security_access(
        &mut self,
        level: u8,
        params: Vec<u8>,
        salt: Vec<u8>,
    ) -> CanResult<(), Self::Error>;
    async fn communication_control(
        &mut self,
        ctrl_type: CommunicationCtrlType,
        comm_type: CommunicationType,
        node_id: Option<request::NodeId>,
        suppress_positive: bool,
        addr_type: AddressType,
    ) -> CanResult<(), Self::Error>;
    #[cfg(feature = "std2020")]
    async fn authentication(
        &mut self,
        auth_task: AuthenticationTask,
        data: request::Authentication,
    ) -> CanResult<response::Authentication, Self::Error>;
    async fn tester_present(
        &mut self,
        r#type: TesterPresentType,
        suppress_positive: bool,
        addr_type: AddressType,
    ) -> CanResult<(), Self::Error>;
    #[cfg(any(feature = "std2006", feature = "std2013"))]
    async fn access_timing_parameter(
        &mut self,
        r#type: TimingParameterAccessType,
        parameter: Vec<u8>,
        suppress_positive: bool,
    ) -> CanResult<Option<response::AccessTimingParameter>, Self::Error>;
    async fn secured_data_transmit(
        &mut self,
        apar: AdministrativeParameter,
        signature: SignatureEncryptionCalculation,
        anti_replay_cnt: u16,
        service: u8,
        service_data: Vec<u8>,
        signature_data: Vec<u8>,
    ) -> CanResult<response::SecuredDataTrans, Self::Error>;
    async fn control_dtc_setting(
        &mut self,
        r#type: DTCSettingType,
        parameter: Vec<u8>,
        suppress_positive: bool,
    ) -> CanResult<(), Self::Error>;
    async fn response_on_event(&mut self) -> CanResult<(), Self::Error>;
    async fn link_control(
        &mut self,
        r#type: LinkCtrlType,
        data: request::LinkCtrl,
        suppress_positive: bool,
    ) -> CanResult<(), Self::Error>;
    async fn read_data_by_identifier(
        &mut self,
        did: DataIdentifier,
        others: Vec<DataIdentifier>,
    ) -> CanResult<response::ReadDID, Self::Error>;
    async fn read_memory_by_address(
        &mut self,
        mem_loc: MemoryLocation,
    ) -> CanResult<Vec<u8>, Self::Error>;
    async fn read_scaling_data_by_identifier(
        &mut self,
        did: DataIdentifier,
    ) -> CanResult<response::ReadScalingDID, Self::Error>;
    /** - Data transmission functional unit - **/
    async fn read_data_by_period_identifier(
        &mut self,
        mode: request::TransmissionMode,
        did: Vec<u8>,
    ) -> CanResult<response::ReadDataByPeriodId, Self::Error>;
    async fn dynamically_define_data_by_identifier(
        &mut self,
        r#type: DefinitionType,
        data: request::DynamicallyDefineDID,
        suppress_positive: bool,
    ) -> CanResult<Option<response::DynamicallyDefineDID>, Self::Error>;
    async fn write_data_by_identifier(
        &mut self,
        did: DataIdentifier,
        data: Vec<u8>,
    ) -> CanResult<(), Self::Error>;
    async fn write_memory_by_address(
        &mut self,
        alfi: AddressAndLengthFormatIdentifier,
        mem_addr: u128,
        mem_size: u128,
        record: Vec<u8>,
    ) -> CanResult<response::WriteMemByAddr, Self::Error>;
    /** Stored data transmission functional unit - **/
    async fn clear_dtc_info(
        &mut self,
        group: utils::U24,
        #[cfg(any(feature = "std2020"))] mem_sel: Option<u8>,
        addr_type: AddressType,
    ) -> CanResult<(), Self::Error>;
    async fn read_dtc_info(
        &mut self,
        r#type: DTCReportType,
        data: request::DTCInfo,
    ) -> CanResult<response::DTCInfo, Self::Error>;
    /** - InputOutput control functional unit - **/
    async fn io_control(
        &mut self,
        did: DataIdentifier,
        param: IOCtrlParameter,
        state: Vec<u8>,
        mask: Vec<u8>,
    ) -> CanResult<response::IOCtrl, Self::Error>;
    /** - Remote activation of routine functional unit - **/
    async fn routine_control(
        &mut self,
        r#type: RoutineCtrlType,
        routine_id: u16,
        option_record: Vec<u8>,
    ) -> CanResult<response::RoutineCtrl, Self::Error>;
    /** - Upload download functional unit - **/
    async fn request_download(
        &mut self,
        alfi: AddressAndLengthFormatIdentifier,
        mem_addr: u128,
        mem_size: u128,
        dfi: Option<DataFormatIdentifier>,
    ) -> CanResult<response::RequestDownload, Self::Error>;
    async fn request_upload(
        &mut self,
        alfi: AddressAndLengthFormatIdentifier,
        mem_addr: u128,
        mem_size: u128,
        dfi: Option<DataFormatIdentifier>,
    ) -> CanResult<response::RequestUpload, Self::Error>;
    async fn transfer_data(
        &mut self,
        sequence: u8,
        data: Vec<u8>,
    ) -> CanResult<response::TransferData, Self::Error>;
    async fn request_transfer_exit(
        &mut self,
        parameter: Vec<u8>,
    ) -> CanResult<Vec<u8>, Self::Error>;
    #[cfg(any(feature = "std2013", feature = "std2020"))]
    async fn request_file_transfer(
        &mut self,
        operation: ModeOfOperation,
        data: request::RequestFileTransfer,
    ) -> CanResult<response::RequestFileTransfer, Self::Error>;
}

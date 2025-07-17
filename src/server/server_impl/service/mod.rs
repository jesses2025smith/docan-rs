/* - Diagnostic and communication management functional unit - */
#[cfg(any(feature = "std2006", feature = "std2013"))] // std2004
mod access_timing_param; // 0x83 ❌
#[cfg(any(feature = "std2020"))]
mod authentication; // 0x29 ❌
mod communication_ctrl; // 0x28 ✅
mod ctrl_dtc_setting; // 0x85 ❌
mod ecu_reset; // 0x11 ✅
mod link_ctrl; // 0x87 ❌
mod response_on_event; // 0x86 ❌
mod secured_data_trans; // 0x84 ❌
mod security_access; // 0x27 ❌
mod session_ctrl; // 0x10 ✅
mod tester_present; // 0x3E ❌

/* - Data transmission functional unit - */
mod dynamically_define_did; // 0x2C ❌
mod read_data_by_pid; // 0x2A ❌
mod read_did; // 0x22 ✅
mod read_mem_by_addr; // 0x23 ❌
mod read_scaling_did; // 0x24 ❌
mod write_did; // 0x2E ✅
mod write_mem_by_addr; // 0x3D ❌

/* - Stored data transmission functional unit - */
mod clear_diagnostic_info; // 0x14 ✅
mod read_dtc_info; // 0x19 ❌

/* - InputOutput control functional unit - */
mod io_ctrl; // 0x2F ❌

/* - Remote activation of routine functional unit - */
mod routine_ctrl; // 0x31 ✅

/* - Upload download functional unit - */
mod request_download; // 0x34 ❌
#[cfg(any(feature = "std2013", feature = "std2020"))]
mod request_file_transfer; // 0x38 ❌
mod request_transfer_exit; // 0x37 ❌
mod request_upload; // 0x35 ❌
mod transfer_data; // 0x36 ❌

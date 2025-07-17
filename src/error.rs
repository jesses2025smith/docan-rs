use iso14229_1::{response::Code, Iso14229Error, Service};
use iso15765_2::IsoTpError;
use rs_can::CanError;

#[derive(thiserror::Error, Debug)]
pub enum DoCanError {
    #[error("{0}")]
    DeviceError(CanError),

    #[error("{0}")]
    Iso14229Error(Iso14229Error),

    #[error("DoCAN - service `{service}` got an unexpected sub-function(expect: {expect}, actual: {actual})")]
    UnexpectedSubFunction {
        service: Service,
        expect: u8,
        actual: u8,
    },

    #[error("DoCAN - service `{expect}` got an unexpect response `{actual}`")]
    UnexpectedResponse { expect: Service, actual: Service },

    #[error("DoCAN - block sequence number of response (0x{actual:02x}) does not match request block sequence number (0x{expect:02x})")]
    UnexpectedTransferSequence { expect: u8, actual: u8 },

    #[error("DoCAN - service `{service}` got a NRC({code:?})")]
    NRCError { service: Service, code: Code },

    #[error("{0}")]
    IsoTpError(IsoTpError),

    #[error("DoCAN - security algorithm error: {0}")]
    SecurityAlgoError(String),

    #[error("DoCAN - other error: {0}")]
    OtherError(String),

    #[error("DoCAN - service: {0} is not implement")]
    NotImplement(Service),
}

impl From<CanError> for DoCanError {
    fn from(error: CanError) -> Self {
        DoCanError::DeviceError(error)
    }
}

impl From<Iso14229Error> for DoCanError {
    fn from(error: Iso14229Error) -> Self {
        DoCanError::Iso14229Error(error)
    }
}

impl From<IsoTpError> for DoCanError {
    fn from(error: IsoTpError) -> Self {
        DoCanError::IsoTpError(error)
    }
}

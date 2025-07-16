//! request of Service 2E

use crate::server::DoCanServer;
use iso14229_1::{
    request::Request,
    response::{Code, Response},
    DataIdentifier, DidConfig, Iso14229Error, Service,
};

impl<D, C, F> DoCanServer<D, C, F> {
    pub(crate) async fn write_did(
        &mut self,
        req: Request,
        cfg: &DidConfig,
    ) -> Result<Response, Iso14229Error> {
        let data = req.raw_data();
        let data_len = data.len();
        match data_len {
            0..=2 => Err(Iso14229Error::InvalidDataLength {
                expect: 3,
                actual: data_len,
            }),
            _ => {
                let did = DataIdentifier::from(u16::from_be_bytes([data[0], data[1]]));
                if self.context.set_static_did(&did, &data[2..]).await {
                    Response::try_from((Service::WriteDID, &data[..2], cfg))
                } else {
                    Ok(Response::new_negative(
                        Service::WriteDID,
                        Code::GeneralReject,
                    ))
                }
            }
        }
    }
}

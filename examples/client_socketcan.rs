use docan_rs::{Client, DoCanClient};
use iso14229_1::{DataIdentifier, SessionType};
use iso15765_2::{Address, AddressType, IsoTp};
use rs_can::{CanDevice, DeviceBuilder};
use rsutil::types::ByteOrder;
use socketcan_rs::SocketCan;
use std::sync::Arc;
use tokio_stream::StreamExt;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let iface = "vcan0".to_string();
    let mut builder = DeviceBuilder::new();
    builder.add_config(iface.clone(), Default::default());

    let mut device = builder.build::<SocketCan>()?;
    let mut client = DoCanClient::new(
        device.clone(),
        iface.clone(),
        Address::default(),
        ByteOrder::default(),
        None,
    )
    .await;
    client.add_data_identifier(DataIdentifier::VIN, 17).await;
    client.iso_tp().start(100).await;

    let isotp = client.iso_tp().clone();
    // create task to process non-uds frame
    let handle = tokio::task::spawn(async move {
        let mut stream = isotp.frame_stream().await.unwrap();
        while let Some(frame) = stream.next().await {
            println!("{}", frame)
        }
    });
    let handle = Arc::new(handle);

    let mut tp_layer = client.iso_tp().clone();
    let mut device_clone = device.clone();
    let handle_weak = Arc::downgrade(&handle);
    let _guard = scopeguard::guard((), |_| {
        futures::executor::block_on(async {
            tp_layer.stop().await;
            device_clone.shutdown();
            if let Some(handle) = handle_weak.upgrade() {
                handle.abort();
            }
        });
    });

    client
        .session_ctrl(SessionType::Extended, false, AddressType::Functional)
        .await?;
    client
        .write_data_by_identifier(DataIdentifier::VIN, "ABCDEF1234567890I".as_bytes().to_vec())
        .await?;

    client.iso_tp().stop().await;
    handle.abort();
    device.shutdown();

    Ok(())
}

use docan_rs::{Client, DoCanClient};
use iso14229_1::SessionType;
use iso15765_2::{Address, AddressType, CanAdapter};
use rs_can::DeviceBuilder;
use rsutil::types::ByteOrder;
use socketcan_rs::SocketCan;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let iface = "vcan0".to_string();
    let mut builder = DeviceBuilder::new();
    builder.add_config(iface.clone(), Default::default());

    let device = builder.build::<SocketCan>()?;
    let mut adapter = CanAdapter::new(device);
    let mut client = DoCanClient::new(adapter.clone(), Some(100));
    client
        .init_channel(iface.clone(), Address::default(), ByteOrder::default())
        .await?;
    adapter.start(100).await;

    client
        .session_ctrl(
            iface.clone(),
            SessionType::Default,
            false,
            AddressType::Functional,
        )
        .await?;

    adapter.stop().await;

    Ok(())
}

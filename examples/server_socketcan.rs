use docan_rs::{DoCanServer, Server};
use iso15765_2::Address;
use rs_can::{CanDevice, DeviceBuilder};
use rsutil::types::ByteOrder;
use socketcan_rs::SocketCan;
use tokio::signal::ctrl_c;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let iface = "vcan0".to_string();
    let mut builder = DeviceBuilder::new();
    builder.add_config(iface.clone(), Default::default());

    let mut device = builder.build::<SocketCan>()?;
    let mut server = DoCanServer::new(
        device.clone(),
        iface.clone(),
        Address {
            tx_id: 0x7E8,
            rx_id: 0x7E0,
            fid: 0x7DF,
        },
        ByteOrder::default(),
    )
    .await;

    server.service_forever(100).await;

    match ctrl_c().await {
        Ok(()) => {
            println!("\n收到 Ctrl+C 信号，程序退出");
            server.service_stop().await;
            device.shutdown();
        }
        Err(err) => {
            eprintln!("监听 Ctrl+C 信号出错: {:?}", err);
        }
    }

    Ok(())
}

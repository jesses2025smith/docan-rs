use docan_rs::{DoCanServer, Server};
use rs_can::{CanDevice, DeviceBuilder};
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
    )
    .await?;

    server.service_forever(100).await;

    match ctrl_c().await {
        Ok(()) => {
            println!("\nCtrl+C Signal, exiting...");
            server.service_stop().await;
            device.shutdown();
        }
        Err(err) => {
            eprintln!("Ctrl+C error: {:?}", err);
        }
    }

    Ok(())
}

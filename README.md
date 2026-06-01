[![Latest version](https://img.shields.io/crates/v/docan.svg)](https://crates.io/crates/docan)
[![Documentation](https://docs.rs/docan/badge.svg)](https://docs.rs/docan)
![LGPL](https://img.shields.io/badge/license-LGPL-green.svg)
![MIT](https://img.shields.io/badge/license-MIT-yellow.svg)

## Overview

DoCAN(Diagnostic Communication over Controller Area Network) 
is a specialized protocol used primarily in automotive and industrial settings.

The driver must implement the CanDriver trait defined in [`rs-can`](https://crates.io/crates/rs-can).

### Implementation status

- Marker legend:
  - `✅` = completed
  - `⭕` = partially completed
  - `❌` = not implemented
- Client services: all currently listed services are implemented (`✅`).
- Server services: most currently listed services are implemented, `ReadDTCInfo (0x19)` is partially completed (`⭕`), and the following services are not implemented (`❌`):
  - `ReadDataByPID (0x2A)`
  - `DynamicallyDefineDID (0x2C)`
  - `ReadMemByAddr (0x23)`
  - `ReadScalingDID (0x24)`
  - `RequestFileTransfer (0x38)`
  - `SecuredDataTrans (0x84)`
  - `ResponseOnEvent (0x86)`

##### [The Server example](examples)
A server configuration file named [docan.server.yaml](docan.server.yaml) 
needs to be added in the same directory as the executable.

```rust
use docan_rs::{DoCanServer, Server};
use rs_can::{CanDevice, DeviceBuilder};
use socketcan_rs::SocketCan;
use std::env;
use tokio::signal::ctrl_c;

fn security_algo(_: u8, seed: &[u8], salt: &[u8]) -> docan_rs::DoCanResult<Option<Vec<u8>>> {
    Ok(Some(
        seed.iter()
            .enumerate()
            .map(|(index, byte)| byte ^ salt[index % salt.len()])
            .collect(),
    ))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let iface = env::var("DOCAN_IFACE").unwrap_or("vcan0".to_string());
    let mut builder = DeviceBuilder::new();
    builder.add_config(iface.clone(), Default::default());

    let mut device = builder.build::<SocketCan>()?;
    let mut server = DoCanServer::new(device.clone(), iface.clone()).await?;
    server.update_security_algo(security_algo).await;

    server.service_forever(100).await;

    match ctrl_c().await {
        Ok(()) => {
            println!("\nCtrl+C Signal, exiting...");
        }
        Err(err) => {
            eprintln!("Ctrl+C error: {:?}", err);
        }
    }

    server.service_stop().await;
    device.shutdown();

    Ok(())
}
```

#### [The client examples](examples)
```rust
use docan_rs::{DoCanClient, DoCanError};
use iso14229_1::{response::Code, DataIdentifier, SessionType, TesterPresentType};
use iso15765_2::{
    can::{Address, AddressType},
    IsoTp,
};
use rs_can::{CanDevice, DeviceBuilder};
use rsutil::types::ByteOrder;
use socketcan_rs::SocketCan;
use std::{env, sync::Arc};
use tokio_stream::StreamExt;

fn security_algo(_: u8, seed: &[u8], salt: &[u8]) -> docan_rs::DoCanResult<Option<Vec<u8>>> {
    Ok(Some(
        seed.iter()
            .enumerate()
            .map(|(index, byte)| byte ^ salt[index % salt.len()])
            .collect(),
    ))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let iface = env::var("DOCAN_IFACE").unwrap_or("vcan0".to_string());
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
    client.update_security_algo(security_algo).await;
    client.tp_layer().start(100).await;

    let tp_layer = client.tp_layer().clone();
    // create task to process non-uds frame
    let handle = tokio::task::spawn(async move {
        let mut stream = tp_layer.frame_stream().await.unwrap();
        while let Some(frame) = stream.next().await {
            println!("{}", frame)
        }
    });
    let handle = Arc::new(handle);

    let mut tp_layer = client.tp_layer().clone();
    let _guard = scopeguard::guard((), |_| {
        futures::executor::block_on(async {
            tp_layer.stop().await;
            device.shutdown();
            handle.abort();
        });
    });

    client
        .session_ctrl(SessionType::Extended, false, AddressType::Physical)
        .await?;

    let vin = "ABCDEF1234567890I".as_bytes();
    match client
        .write_data_by_identifier(DataIdentifier::VIN, vin.to_vec())
        .await
    {
        Err(DoCanError::NRCError {
            service: _,
            code: Code::SecurityAccessDenied,
        }) => {}
        Err(err) => return Err(err.into()),
        Ok(()) => anyhow::bail!("write_data_by_identifier should require security access"),
    }

    client
        .unlock_security_access(3, vec![], vec![0x01, 0x02, 0x03, 0x04])
        .await?;
    client
        .write_data_by_identifier(DataIdentifier::VIN, vin.to_vec())
        .await?;

    let data = client
        .read_data_by_identifier(DataIdentifier::VIN, vec![])
        .await?
        .data;
    assert_eq!(data.did, DataIdentifier::VIN);
    assert_eq!(data.data, vin);

    client
        .tester_present(TesterPresentType::Zero, false, AddressType::Physical)
        .await?;

    Ok(())
}
```

### Prerequisites

- Rust 1.70 or higher
- Cargo (included with Rust)

## Contributing

We're always looking for users who have thoughts on how to make `docan` better, or users with
interesting use cases.  Of course, we're also happy to accept code contributions for outstanding
feature requests!

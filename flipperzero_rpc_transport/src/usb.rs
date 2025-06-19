use std::sync::Arc;

use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf},
    sync::Mutex,
};
use tokio_serial::{SerialPortBuilderExt, SerialStream};
use tokio_util::codec::{FramedRead, FramedWrite};

use crate::{FlipperZeroRpcTransport, codec::FlipperZeroRpcCodec};
use tokio::time::{Duration, timeout};

const FLIPPER_BAUD_RATE: u32 = 115200;
const CLI_PROMPT: [u8; 4] = *b"\n>: ";
const START_RPC_COMMAND: [u8; 18] = *b"start_rpc_session\n";

pub struct UsbTransport {
    port_rx: Arc<Mutex<FramedRead<ReadHalf<SerialStream>, FlipperZeroRpcCodec>>>,
    port_tx: Arc<Mutex<FramedWrite<WriteHalf<SerialStream>, FlipperZeroRpcCodec>>>,
}

impl UsbTransport {
    pub async fn new(path: &str) -> Result<Self, tokio_serial::Error> {
        let mut port = tokio_serial::new(path, FLIPPER_BAUD_RATE).open_native_async()?;

        // Wait for the CLI prompt to appear
        wait_for_pattern(&mut port, &CLI_PROMPT).await?;

        // Start the RPC session
        {
            let mut written = 0;

            while written < START_RPC_COMMAND.len() {
                written += port.write(&START_RPC_COMMAND[written..]).await?;
            }

            port.flush().await?;
        }

        // And wait for the terminal echo for our command
        wait_for_pattern(&mut port, &START_RPC_COMMAND).await?;

        let (rx, tx) = tokio::io::split(port);

        Ok(UsbTransport {
            port_rx: Arc::new(Mutex::new(FramedRead::new(
                rx,
                FlipperZeroRpcCodec::default(),
            ))),
            port_tx: Arc::new(Mutex::new(FramedWrite::new(
                tx,
                FlipperZeroRpcCodec::default(),
            ))),
        })
    }
}

async fn wait_for_pattern(
    port: &mut SerialStream,
    pattern: &[u8],
) -> Result<(), tokio_serial::Error> {
    let inner = async move {
        let mut recent_bytes: Vec<u8> = vec![];
        let mut read_buffer = [0u8; 1024];

        loop {
            // TODO: this *may* read too much data from the port, which can cause loss of RPC data
            let bytes_read = port.read(&mut read_buffer).await?;
            recent_bytes.extend_from_slice(&read_buffer[0..bytes_read]);

            if recent_bytes.len() > 32 {
                recent_bytes.drain(0..(recent_bytes.len() - 32));
            }

            if recent_bytes
                .windows(pattern.len())
                .any(|window| window == pattern)
            {
                return Ok(());
            }
        }
    };

    timeout(Duration::from_secs(5), inner).await.map_err(|_| {
        tokio_serial::Error::new(
            tokio_serial::ErrorKind::Io(std::io::ErrorKind::TimedOut),
            "Timed out waiting for pattern",
        )
    })?
}

#[async_trait]
impl FlipperZeroRpcTransport for UsbTransport {
    async fn write_frame(&mut self, data: &[u8]) -> Result<(), crate::error::Error> {
        let mut port_tx = self.port_tx.lock().await;
        Ok(port_tx.send(data).await?)
    }

    async fn read_frame(&mut self) -> Result<Vec<u8>, crate::error::Error> {
        let mut port_rx = self.port_rx.lock().await;
        match port_rx.next().await {
            Some(x) => Ok(x?),
            None => Err(
                std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "No data received").into(),
            ),
        }
    }
}

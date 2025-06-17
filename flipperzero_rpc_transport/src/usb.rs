use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_serial::{SerialPortBuilderExt, SerialStream};
use tokio_util::codec::Framed;

use crate::codec::FlipperZeroRpcCodec;
use tokio::time::{Duration, timeout};

const FLIPPER_BAUD_RATE: u32 = 115200;
const CLI_PROMPT: [u8; 4] = *b"\n>: ";
const START_RPC_COMMAND: [u8; 18] = *b"start_rpc_session\n";

pub struct UsbTransport {
    port: Framed<SerialStream, FlipperZeroRpcCodec>,
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

        Ok(UsbTransport {
            port: Framed::new(port, FlipperZeroRpcCodec::default()),
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

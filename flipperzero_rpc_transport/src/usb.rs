use futures_util::{SinkExt, StreamExt};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf},
    sync::{broadcast, mpsc},
};
use tokio_serial::{SerialPortBuilderExt, SerialStream};
use tokio_util::codec::{FramedRead, FramedWrite};

use crate::{CallbackChannel, FzRpcTransport, codec::FzRpcCodec};
use tokio::time::{Duration, timeout};

const FLIPPER_BAUD_RATE: u32 = 115200;
const CLI_PROMPT: [u8; 4] = *b"\n>: ";
const START_RPC_COMMAND: [u8; 19] = *b"start_rpc_session\r\n";

pub struct UsbTransport {
    rx_sender: broadcast::WeakSender<Vec<u8>>,
    tx_sender: mpsc::UnboundedSender<(Vec<u8>, Option<CallbackChannel>)>,

    rx_task_handle: tokio::task::JoinHandle<()>,
    tx_task_handle: tokio::task::JoinHandle<()>,
}

impl UsbTransport {
    pub async fn new(path: &str) -> Result<Self, tokio_serial::Error> {
        let mut port = tokio_serial::new(path, FLIPPER_BAUD_RATE).open_native_async()?;

        // Wait for the CLI prompt to appear
        wait_for_pattern(&mut port, &CLI_PROMPT).await?;

        // Start the RPC session
        {
            let mut written = 0;

            // We have to exclude the \n at the end of the command for... whatever reason.
            // Else it would end up being the first byte of the first RPC frame.
            while written < START_RPC_COMMAND.len() - 1 {
                written += port
                    .write(&START_RPC_COMMAND[written..(START_RPC_COMMAND.len() - 1)])
                    .await?;
            }

            port.flush().await?;
        }

        // And wait for the terminal echo for our command
        wait_for_pattern(&mut port, &START_RPC_COMMAND).await?;

        let (port_rx, port_tx) = tokio::io::split(port);

        let port_rx = FramedRead::new(port_rx, FzRpcCodec::default());
        let port_tx = FramedWrite::new(port_tx, FzRpcCodec::default());

        let rx_sender = broadcast::Sender::new(32);
        let (tx_sender, tx_receiver) = mpsc::unbounded_channel();

        let rx_task_handle = tokio::spawn(Self::rx_task(port_rx, rx_sender.clone()));
        let rx_sender = rx_sender.downgrade();

        let tx_task_handle = tokio::spawn(Self::tx_task(port_tx, tx_receiver));

        Ok(UsbTransport {
            rx_sender,
            tx_sender,

            rx_task_handle,
            tx_task_handle,
        })
    }

    async fn rx_task(
        mut port_rx: FramedRead<ReadHalf<SerialStream>, FzRpcCodec>,
        rx_sender: broadcast::Sender<Vec<u8>>,
    ) {
        while let Some(frame) = port_rx.next().await {
            match frame {
                Ok(data) => {
                    // We don't care about the error, because it means no one is currently subscribed to the channel, which is fine.
                    rx_sender.send(data).ok();
                }
                Err(_) => {
                    // Because [`FzRpcCodec`]'s implementation never returns an error,
                    // we can only end up here if [`FramedRead`] returns an error,
                    // which can only happen if we've reached an EOF and tried to read more data,
                    // which itself will never happen, because we would break out of the loop.
                    unreachable!(
                        "Unexpected error in rx_task: FramedRead returned an error, which should not happen"
                    );
                }
            }
        }

        // Got an EOF - the channel will close automatically, beause the last "hard" sender will be dropped
    }

    async fn tx_task(
        mut port_tx: FramedWrite<WriteHalf<SerialStream>, FzRpcCodec>,
        mut tx_receiver: mpsc::UnboundedReceiver<(Vec<u8>, Option<CallbackChannel>)>,
    ) {
        while let Some((data, callback)) = tx_receiver.recv().await {
            let res = port_tx.send(&data).await;
            if let Some(callback) = callback {
                callback.send(res.map(|_| data).map_err(|e| e.into())).ok();
            }
        }

        // The channel has been closed (the transport and all other senders have been dropped).
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
            format!(
                "Timed out waiting for pattern: `{}`",
                String::from_utf8_lossy(pattern)
            ),
        )
    })?
}

impl FzRpcTransport for UsbTransport {
    fn rx(&self) -> Option<broadcast::Receiver<Vec<u8>>> {
        self.rx_sender.upgrade().map(|sender| sender.subscribe())
    }

    fn tx(&self) -> mpsc::UnboundedSender<(Vec<u8>, Option<CallbackChannel>)> {
        self.tx_sender.clone()
    }
}

impl Drop for UsbTransport {
    fn drop(&mut self) {
        self.rx_task_handle.abort();
        self.tx_task_handle.abort();

        assert_eq!(self.rx_sender.strong_count(), 0);
    }
}

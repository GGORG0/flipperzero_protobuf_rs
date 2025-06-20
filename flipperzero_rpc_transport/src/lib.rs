pub(crate) mod codec;
pub mod error;
pub mod usb;

use async_trait::async_trait;
use tokio::sync::{broadcast, mpsc, oneshot};

pub type CallbackChannel = oneshot::Sender<Result<(), crate::error::Error>>;

pub trait FzRpcTransport {
    /// Subscribe to the transport's receive channel.
    fn rx(&self) -> Option<broadcast::Receiver<Vec<u8>>>;

    /// Create a sender to send data to the transport.
    fn tx(&self) -> mpsc::UnboundedSender<(Vec<u8>, Option<CallbackChannel>)>;
}

#[async_trait]
pub trait FzRpcTransportExt {
    /// Read a single frame from the transport.
    async fn read(&self) -> Result<Vec<u8>, crate::error::Error>;

    /// Write a single frame to the transport.
    async fn write(&self, data: Vec<u8>) -> Result<(), crate::error::Error>;
}

#[async_trait]
impl<T: FzRpcTransport + Send + Sync> FzRpcTransportExt for T {
    async fn read(&self) -> Result<Vec<u8>, crate::error::Error> {
        let mut rx = self.rx().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "The transport has been closed",
            )
        })?;

        // If the channel was closed, we would have returned an error above.
        let data = rx.recv().await.unwrap();

        Ok(data)
    }

    async fn write(&self, data: Vec<u8>) -> Result<(), crate::error::Error> {
        let tx = self.tx();

        let (callback_tx, callback_rx) = oneshot::channel();

        tx.send((data, Some(callback_tx))).map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "The transport has been closed",
            )
        })?;

        callback_rx.await.map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "The callback channel has been closed",
            )
        })?
    }
}

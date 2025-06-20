pub(crate) mod codec;
pub mod error;
pub mod usb;

use tokio::sync::{broadcast, mpsc, oneshot};

pub type CallbackChannel = oneshot::Sender<Result<(), crate::error::Error>>;

pub trait FzRpcTransport {
    /// Subscribe to the transport's receive channel.
    fn rx(&self) -> Option<broadcast::Receiver<Vec<u8>>>;

    /// Create a sender to send data to the transport.
    fn tx(&self) -> mpsc::UnboundedSender<(Vec<u8>, Option<CallbackChannel>)>;
}

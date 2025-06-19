pub(crate) mod codec;
pub mod error;
pub mod usb;

use async_trait::async_trait;

use crate::error::Error;

#[async_trait]
pub trait FlipperZeroRpcTransport {
    /// Write a single frame of data to the transport.
    async fn write_frame(&self, data: &[u8]) -> Result<(), Error>;

    /// Read a single frame of data from the transport.
    async fn read_frame(&self) -> Result<Vec<u8>, Error>;
}

use bytes::{Buf, BufMut, BytesMut};
use integer_encoding::VarInt;
use std::io::{Error, Result};
use tokio_util::codec::{Decoder, Encoder};

#[derive(Default)]
pub(crate) struct FlipperZeroRpcCodec {
    buf: Vec<u8>,
}

impl Decoder for FlipperZeroRpcCodec {
    type Item = Vec<u8>;
    type Error = Error;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<Vec<u8>>> {
        self.buf.extend_from_slice(buf);
        buf.advance(buf.len());

        match u64::decode_var(&self.buf) {
            Some((frame_len, header_len)) => {
                if self.buf.len() >= frame_len as usize + header_len {
                    self.buf.drain(0..header_len);
                    let frame = self.buf.drain(0..frame_len as usize).collect();

                    Ok(Some(frame))
                } else {
                    Ok(None)
                }
            }
            None => Ok(None),
        }
    }
}

impl Encoder<&[u8]> for FlipperZeroRpcCodec {
    type Error = Error;

    fn encode(&mut self, data: &[u8], buf: &mut BytesMut) -> Result<()> {
        let mut header = [0u8; 8];

        let header_len = (data.len() as u64).encode_var(&mut header);
        buf.put_slice(&header[..header_len]);

        buf.put_slice(data);

        Ok(())
    }
}

use flipperzero_protobuf_raw::{
    pb::{
        main::Content,
        {CommandStatus, Main},
    },
    prost::Message,
};

use crate::FzRpcTransport;
use std::borrow::{Borrow, BorrowMut};
use std::ops::{Deref, DerefMut};

pub use flipperzero_protobuf_raw;

pub struct FzRpcProto<T: FzRpcTransport> {
    transport: T,

    next_command_id: u32,
}

impl<T: FzRpcTransport> FzRpcProto<T> {
    pub fn new(transport: T) -> Self {
        Self {
            transport,
            next_command_id: 0,
        }
    }

    fn get_command_id(&mut self) -> u32 {
        let current = self.next_command_id;
        self.next_command_id += 1;
        current
    }

    pub async fn send(&mut self, content: Content) -> Result<Vec<u8>, crate::error::Error> {
        self.send_advanced(content, None, None, None).await
    }

    pub async fn send_advanced(
        &mut self,
        content: Content,
        has_next: Option<bool>,
        command_id: Option<u32>,
        command_status: Option<CommandStatus>,
    ) -> Result<Vec<u8>, crate::error::Error> {
        let command_id = command_id.unwrap_or_else(|| self.get_command_id());

        let command = Main {
            command_id,
            command_status: command_status.unwrap_or(CommandStatus::Ok).into(),
            has_next: has_next.unwrap_or(false),
            content: Some(content),
        };

        let data = command.encode_to_vec();

        self.transport.write(data).await
    }
}

impl<T: FzRpcTransport> Deref for FzRpcProto<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.transport
    }
}

impl<T: FzRpcTransport> DerefMut for FzRpcProto<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.transport
    }
}

impl<T: FzRpcTransport> AsRef<T> for FzRpcProto<T> {
    fn as_ref(&self) -> &T {
        &self.transport
    }
}

impl<T: FzRpcTransport> AsMut<T> for FzRpcProto<T> {
    fn as_mut(&mut self) -> &mut T {
        &mut self.transport
    }
}

impl<T: FzRpcTransport> Borrow<T> for FzRpcProto<T> {
    fn borrow(&self) -> &T {
        &self.transport
    }
}

impl<T: FzRpcTransport> BorrowMut<T> for FzRpcProto<T> {
    fn borrow_mut(&mut self) -> &mut T {
        &mut self.transport
    }
}

impl<T: FzRpcTransport> From<T> for FzRpcProto<T> {
    fn from(transport: T) -> Self {
        FzRpcProto::new(transport)
    }
}

use flipperzero_rpc_transport::{
    FzRpcTransport,
    proto::{
        FzRpcProto,
        flipperzero_protobuf_raw::{
            pb::main::Content,
            pb_system::{RebootRequest, reboot_request::RebootMode},
        },
    },
    usb::UsbTransport,
};

#[tokio::main]
async fn main() {
    let usb = UsbTransport::new("/dev/ttyACM0").await.unwrap();

    let mut proto = FzRpcProto::new(usb);

    let sent = proto
        .send(Content::SystemRebootRequest(RebootRequest {
            mode: RebootMode::Dfu.into(),
        }))
        .await
        .unwrap();

    println!("Sent: {}", hex::encode(&sent));

    let mut rx = proto.rx().unwrap();

    while let Ok(data) = rx.recv().await {
        println!("Received: {}", hex::encode(&data));
    }
}

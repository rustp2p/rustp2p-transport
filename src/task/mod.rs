use std::io;

use bytes::BytesMut;
use rustp2p::pipe::{Pipe, PipeLine, PipeWriter, RecvError};
use tcp_ip::ip_stack::{IpStackRecv, IpStackSend};

pub async fn start(
    mtu: u16,
    pipe: Pipe,
    ip_stack_send: IpStackSend,
    ip_stack_recv: IpStackRecv,
) -> io::Result<()> {
    if pipe.writer().pipe_context().load_id().is_none() {
        return Err(io::Error::new(io::ErrorKind::Other, "not node id"));
    };
    let pipe_writer = pipe.writer().clone();
    tokio::spawn(async move {
        if let Err(e) = pipe_accept_handle(pipe, ip_stack_send).await {
            if e.kind() != io::ErrorKind::BrokenPipe {
                log::warn!("pipe_accept {e:?}");
            }
        }
    });
    tokio::spawn(async move {
        ip_stack_recv_handle(mtu as usize, ip_stack_recv, &pipe_writer).await;
        _ = pipe_writer.shutdown();
    });
    Ok(())
}

async fn ip_stack_recv_handle(
    mtu: usize,
    mut ip_stack_recv: IpStackRecv,
    pipe_writer: &PipeWriter,
) {
    let mut bufs = Vec::with_capacity(128);
    let mut sizes = vec![0; 128];
    for _ in 0..128 {
        bufs.push(BytesMut::zeroed(mtu))
    }
    while let Ok(num) = ip_stack_recv.recv_ip_packet(&mut bufs, &mut sizes).await {
        for index in 0..num {
            let buf = &bufs[index];
            let len = sizes[index];
            let packet = pnet_packet::ipv4::Ipv4Packet::new(&buf[..len]).unwrap();
            let dst = packet.get_destination();
            let mut send_packet = pipe_writer.allocate_send_packet();
            send_packet.set_payload(buf);
            if let Err(e) = pipe_writer.send_packet_to(send_packet, &dst.into()).await {
                log::warn!("send_packet_to {e:?},dst={dst}");
            }
        }
    }
}

async fn pipe_accept_handle(mut pipe: Pipe, ip_stack_send: IpStackSend) -> io::Result<()> {
    loop {
        let line = pipe.accept().await?;
        let ip_stack_send = ip_stack_send.clone();
        tokio::spawn(async move {
            let addr = line.remote_addr();
            if let Err(e) = pipe_line_recv_handle(line, ip_stack_send).await {
                log::warn!("pipe_line_recv {e:?} {addr:?}");
            }
        });
    }
}
async fn pipe_line_recv_handle(
    mut pipe_line: PipeLine,
    ip_stack_send: IpStackSend,
) -> io::Result<()> {
    let mut list = Vec::with_capacity(16);

    loop {
        let result = match pipe_line.recv_multi(&mut list).await {
            Ok(rs) => rs,
            Err(e) => {
                return match e {
                    RecvError::Done => Ok(()),
                    RecvError::Io(e) => Err(e),
                }
            }
        };
        if let Err(e) = result {
            log::warn!("recv_data_handle {e:?} {:?}", pipe_line.remote_addr());
            continue;
        }
        for data in list.drain(..) {
            if let Err(e) = ip_stack_send.send_ip_packet(data.payload()).await {
                log::warn!("{e:?} {:?}", data.route_key());
            }
        }
    }
}

mod task;

pub use rustp2p;
use std::io;
pub use tcp_ip;
pub async fn transport(
    rustp2p_config: rustp2p::config::PipeConfig,
    tcp_ip_config: tcp_ip::IpStackConfig,
) -> io::Result<tcp_ip::IpStack> {
    let mtu = tcp_ip_config.mtu;
    let pipe = rustp2p::pipe::Pipe::new(rustp2p_config).await?;
    let (ip_stack, ip_stack_send, ip_stack_recv) = tcp_ip::ip_stack(tcp_ip_config)?;
    task::start(mtu, pipe, ip_stack_send, ip_stack_recv).await?;
    Ok(ip_stack)
}

mod task;

pub use rustp2p;
use rustp2p::pipe::PeerNodeAddress;
use rustp2p::protocol::node_id::GroupCode;
use std::io;
use std::net::Ipv4Addr;
pub use tcp_ip;
/// A builder for configuring and creating a `Transport` instance.
/// Supports setting IP, port, peers, and group code for network isolation.
#[derive(Clone, Debug, Default)]
pub struct TransportBuilder {
    group_code: Option<String>,
    ip: Option<Ipv4Addr>,
    port: Option<u16>,
    peers: Option<Vec<PeerNodeAddress>>,
}
impl TransportBuilder {
    /// Sets the group code for network isolation.
    pub fn group_code(mut self, group_code: String) -> Self {
        self.group_code = Some(group_code);
        self
    }
    /// Sets the IP address of the node (must be set).
    pub fn ip(mut self, ip: Ipv4Addr) -> Self {
        self.ip = Some(ip);
        self
    }
    /// Sets the port for the node to listen on (optional).
    pub fn port(mut self, port: u16) -> Self {
        self.port = Some(port);
        self
    }
    /// Sets the list of directly connected peer nodes (optional).
    pub fn peers(mut self, peers: Vec<PeerNodeAddress>) -> Self {
        self.peers = Some(peers);
        self
    }
    pub async fn build(self) -> io::Result<tcp_ip::IpStack> {
        let Some(ip) = self.ip else {
            return Err(io::Error::new(io::ErrorKind::Other, "IP must be set"));
        };
        let mut udp_config = rustp2p::config::UdpPipeConfig::default();
        let mut tcp_config = rustp2p::config::TcpPipeConfig::default();
        if let Some(port) = self.port {
            udp_config = udp_config.set_simple_udp_port(port);
            tcp_config = tcp_config.set_tcp_port(port);
        }
        let mut p2p_config = rustp2p::config::PipeConfig::empty()
            .set_udp_pipe_config(udp_config)
            .set_tcp_pipe_config(tcp_config)
            .set_node_id(ip.into());
        if let Some(peers) = self.peers {
            p2p_config = p2p_config.set_direct_addrs(peers);
        }
        if let Some(group_code) = self.group_code {
            p2p_config = p2p_config.set_group_code(string_to_group_code(&group_code))
        }

        let ip_stack_config = tcp_ip::IpStackConfig::default();
        transport_from_config(p2p_config, ip_stack_config).await
    }
}

/// Build through complete config
pub async fn transport_from_config(
    rustp2p_config: rustp2p::config::PipeConfig,
    tcp_ip_config: tcp_ip::IpStackConfig,
) -> io::Result<tcp_ip::IpStack> {
    let mtu = tcp_ip_config.mtu;
    let pipe = rustp2p::pipe::Pipe::new(rustp2p_config).await?;
    let (ip_stack, ip_stack_send, ip_stack_recv) = tcp_ip::ip_stack(tcp_ip_config)?;
    task::start(mtu, pipe, ip_stack_send, ip_stack_recv).await?;
    Ok(ip_stack)
}
fn string_to_group_code(input: &str) -> GroupCode {
    let mut array = [0u8; 16];
    let bytes = input.as_bytes();
    let len = bytes.len().min(16);
    array[..len].copy_from_slice(&bytes[..len]);
    array.into()
}

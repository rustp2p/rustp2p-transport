mod task;

pub use rustp2p;
use rustp2p::config::LoadBalance;
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
    endpoint: Option<Ipv4Addr>,
    load_balance: Option<LoadBalance>,
    listen_port: Option<u16>,
    peers: Option<Vec<PeerNodeAddress>>,
}
impl TransportBuilder {
    /// Sets the group code for network isolation.
    pub fn group_code(mut self, group_code: String) -> Self {
        self.group_code = Some(group_code);
        self
    }
    /// Sets the IP address of the node (must be set).
    pub fn endpoint(mut self, endpoint: Ipv4Addr) -> Self {
        self.endpoint = Some(endpoint);
        self
    }
    /// Sets the port for the node to listen on (optional).
    pub fn listen_port(mut self, listen_port: u16) -> Self {
        self.listen_port = Some(listen_port);
        self
    }
    /// Sets the list of directly connected peer nodes (optional).
    pub fn peers(mut self, peers: Vec<PeerNodeAddress>) -> Self {
        self.peers = Some(peers);
        self
    }
    pub fn load_balance(mut self, load_balance: LoadBalance) -> Self {
        self.load_balance = Some(load_balance);
        self
    }
    fn config(self) -> io::Result<(rustp2p::config::PipeConfig, tcp_ip::IpStackConfig)> {
        let Some(ip) = self.endpoint else {
            return Err(io::Error::new(io::ErrorKind::Other, "IP must be set"));
        };
        let mut udp_config = rustp2p::config::UdpPipeConfig::default();
        let mut tcp_config = rustp2p::config::TcpPipeConfig::default();
        if let Some(port) = self.listen_port {
            udp_config = udp_config.set_simple_udp_port(port);
            tcp_config = tcp_config.set_tcp_port(port);
        }
        let mut p2p_config = rustp2p::config::PipeConfig::empty()
            .set_load_balance(self.load_balance.unwrap_or(LoadBalance::RoundRobin))
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
        Ok((p2p_config, ip_stack_config))
    }
    #[cfg(feature = "global")]
    pub async fn build_context(self) -> io::Result<()> {
        let (p2p_config, ip_stack_config) = self.config()?;
        transport_from_config(p2p_config, ip_stack_config).await
    }
    #[cfg(not(feature = "global"))]
    pub async fn build(self) -> io::Result<tcp_ip::IpStack> {
        let (p2p_config, ip_stack_config) = self.config()?;
        transport_from_config(p2p_config, ip_stack_config).await
    }
}
/// Build through complete config
#[cfg(feature = "global")]
pub async fn transport_from_config(
    rustp2p_config: rustp2p::config::PipeConfig,
    tcp_ip_config: tcp_ip::IpStackConfig,
) -> io::Result<()> {
    transport_from_config0(rustp2p_config, tcp_ip_config)
        .await
        .map(|_| ())
}
/// Build through complete config
#[cfg(not(feature = "global"))]
pub async fn transport_from_config(
    rustp2p_config: rustp2p::config::PipeConfig,
    tcp_ip_config: tcp_ip::IpStackConfig,
) -> io::Result<tcp_ip::IpStack> {
    transport_from_config0(rustp2p_config, tcp_ip_config).await
}
async fn transport_from_config0(
    rustp2p_config: rustp2p::config::PipeConfig,
    tcp_ip_config: tcp_ip::IpStackConfig,
) -> io::Result<tcp_ip::IpStack> {
    let mtu = tcp_ip_config.mtu;
    let pipe = rustp2p::pipe::Pipe::new(rustp2p_config).await?;
    let Some(node_id) = pipe.writer().pipe_context().load_id() else {
        return Err(io::Error::new(io::ErrorKind::Other, "Node ID must be set"));
    };
    #[cfg(not(feature = "global"))]
    let (ip_stack, ip_stack_send, ip_stack_recv) = tcp_ip::ip_stack(tcp_ip_config)?;
    #[cfg(feature = "global")]
    let (ip_stack_send, ip_stack_recv) = tcp_ip::ip_stack(tcp_ip_config)?;
    #[cfg(feature = "global")]
    let ip_stack = tcp_ip::IpStack::get()?;
    ip_stack.routes().set_default_v4(node_id.into());
    let mut v6: [u8; 16] = [
        0xfd, 0x12, 0x34, 0x56, 0x78, 0x9a, 0xab, 0xcd, 0xef, 0x12, 0x34, 0x56, 0, 0, 0, 0,
    ];
    v6[12..].copy_from_slice(node_id.as_ref());
    ip_stack
        .routes()
        .set_default_v6(std::net::Ipv6Addr::from(v6));
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

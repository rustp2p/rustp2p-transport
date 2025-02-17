Getting Started
------
The nodes in the `rustp2p-transport` are decentralized, and all nodes are equal.
This demonstrates a simple implementation where Node B accesses Node A.

Node-A(192.168.0.2):

```rust
use rustp2p::config::{PipeConfig, TcpPipeConfig, UdpPipeConfig};
use tcp_ip::IpStackConfig;
use tokio::io::AsyncReadExt;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let ip = std::net::Ipv4Addr::new(10, 0, 0, 2);
    let port = 12345;
    let udp_config = UdpPipeConfig::default().set_udp_ports(vec![port]);
    let tcp_config = TcpPipeConfig::default().set_tcp_port(port);
    let p2p_config = PipeConfig::empty()
        .set_udp_pipe_config(udp_config)
        .set_tcp_pipe_config(tcp_config)
        .set_node_id(ip.into());
    let ip_stack_config = IpStackConfig::default();
    let ip_stack = rustp2p_transport::transport(p2p_config, ip_stack_config).await?;
    let mut tcp_listener = tcp_ip::tcp::TcpListener::bind(ip_stack.clone(), "0.0.0.0:8080").await?;
    println!("***** tcp_listener accept *****");
    loop {
        let (mut stream, addr) = tcp_listener.accept().await?;
        println!("***** accept stream {addr} *****");
        tokio::spawn(async move {
            let mut buf = vec![0; 65536];
            loop {
                match stream.read(&mut buf).await {
                    Ok(n) => {
                        println!("{addr}: {:?}", String::from_utf8(buf[..n].to_vec()));
                    }
                    Err(e) => {
                        println!("error addr={addr} {e:?}");
                        return;
                    }
                }
            }
        });
    }
}
```

Node-B:

```rust
use rustp2p::config::{PipeConfig, TcpPipeConfig, UdpPipeConfig};
use tcp_ip::IpStackConfig;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let node_a = std::net::Ipv4Addr::new(10, 0, 0, 2);
    let node_b = std::net::Ipv4Addr::new(10, 0, 0, 3);
    let port = 12345;
    let udp_config = UdpPipeConfig::default().set_udp_ports(vec![port]);
    let tcp_config = TcpPipeConfig::default().set_tcp_port(port);
    let p2p_config = PipeConfig::empty()
        .set_udp_pipe_config(udp_config)
        .set_tcp_pipe_config(tcp_config)
        .set_direct_addrs(vec!["tcp://192.168.0.2:12345".parse().unwrap()])
        .set_node_id(node_b.into());
    let ip_stack_config = IpStackConfig::default();
    let ip_stack = rustp2p_transport::transport(p2p_config, ip_stack_config).await?;
    let tcp_stream = tcp_ip::tcp::TcpStream::bind(ip_stack, format!("{node_b}:8081"))?
        .connect(format!("{node_a}:8080"))
        .await?;
    // Use `tcp_stream` to communicate with Node A.
    Ok(())
}
```
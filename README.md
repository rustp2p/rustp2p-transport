# rustp2p-transport

`rustp2p-transport` creates a decentralized communication platform.
It organizes network nodes into a virtual LAN,
allowing users to communicate through TCP/UDP APIs similar to those in the standard library.


Getting Started
------
The nodes in the `rustp2p-transport` are decentralized, and all nodes are equal.
This demonstrates a simple implementation where Node B accesses Node A.

`Cargo.toml`:

```toml
[dependencies]
rustp2p-transport = {version = "0.1", features = ["global"]}
```
Node-A(192.168.0.2):

```rust
use tokio::io::AsyncReadExt;
use rustp2p_transport::TransportBuilder;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let ip = std::net::Ipv4Addr::new(10, 0, 0, 2);
    let port = 12345;
    TransportBuilder::default()
        .endpoint(ip)
        .listen_port(port)
        .build_context()
        .await?;
    let mut tcp_listener = tcp_ip::tcp::TcpListener::bind("0.0.0.0:8080").await?;
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
use rustp2p_transport::TransportBuilder;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let node_a = std::net::Ipv4Addr::new(10, 0, 0, 2);
    let node_b = std::net::Ipv4Addr::new(10, 0, 0, 3);
    let port = 12345;
    TransportBuilder::default()
        .endpoint(node_b)
        .listen_port(port)
        .peers(vec!["tcp://192.168.0.2:12345".parse().unwrap()])
        .build_context()
        .await?;
    let tcp_stream = tcp_ip::tcp::TcpStream::connect(format!("{node_a}:8080")).await?;
    // Use `tcp_stream` to communicate with Node A.
    Ok(())
}
```
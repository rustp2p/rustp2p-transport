use clap::Parser;
use env_logger::Env;
use rustp2p::pipe::PeerNodeAddress;
use rustp2p_transport::TransportBuilder;
use std::io;
use std::net::Ipv4Addr;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Peer node address.
    /// example: --peer tcp://192.168.10.13:23333 --peer udp://192.168.10.23:23333
    #[arg(short, long)]
    peer: Option<Vec<String>>,
    /// Set node IP
    /// example: --ip 10.26.0.2
    #[arg(short, long)]
    ip: Ipv4Addr,
    /// Connect node IP
    /// example: --connect 10.26.0.3
    #[arg(short, long)]
    connect: Option<Ipv4Addr>,
    /// Listen local port
    #[arg(short = 'P', long, default_value = "23333")]
    port: u16,
}

/// Node A: ./connect --ip 10.26.0.2 -P 23333
/// Node B: ./connect --ip 10.26.0.3 -P 23334 -c 10.26.0.2 --peer tcp://127.0.0.1:23333
/// After a successful connection, entering content on Node B will be displayed on Node A.
#[tokio::main]
async fn main() -> io::Result<()> {
    main0().await
}

pub async fn main0() -> io::Result<()> {
    let Args {
        peer,
        ip,
        connect,
        port,
    } = Args::parse();
    env_logger::Builder::from_env(Env::default().default_filter_or("debug")).init();
    let mut addrs = Vec::new();
    if let Some(peers) = peer {
        for addr in peers {
            addrs.push(addr.parse::<PeerNodeAddress>().expect("--peer"))
        }
    }
    let ip_stack = TransportBuilder::default()
        .ip(ip)
        .port(port)
        .peers(addrs)
        .build()
        .await?;

    // Nodes can communicate using network protocols
    if let Some(connect) = connect {
        log::info!("***** connect stream {connect}");
        let mut tcp_stream = tcp_ip::tcp::TcpStream::bind(ip_stack, format!("{ip}:12345"))?
            .connect(format!("{connect}:12345"))
            .await?;
        let mut reader = BufReader::new(tokio::io::stdin());
        let mut string = String::new();
        loop {
            string.clear();
            println!(
                "========================== Please enter the char: =========================="
            );
            reader.read_line(&mut string).await?;
            println!("input: {}", string);
            tcp_stream.write_all(string.as_bytes()).await?;
        }
    } else {
        let mut tcp_listener =
            tcp_ip::tcp::TcpListener::bind(ip_stack.clone(), "0.0.0.0:12345").await?;
        log::info!("***** tcp_listener accept");
        loop {
            let (mut stream, addr) = tcp_listener.accept().await?;
            log::info!("***** accept stream {addr}");
            tokio::spawn(async move {
                let mut buf = vec![0; 65536];
                loop {
                    let rs = match tokio::time::timeout(Duration::from_secs(5), stream.read(&mut buf)).await {
                        Ok(rs) => rs,
                        Err(_) => {
                            log::info!("timeout {addr}");
                            return;
                        }
                    };
                    match rs {
                        Ok(n) => {
                            log::info!("{addr}: {:?}", String::from_utf8(buf[..n].to_vec()));
                        }
                        Err(e) => {
                            log::error!("addr={addr} {e:?}");
                            return;
                        }
                    }
                }
            });
        }
    }
}

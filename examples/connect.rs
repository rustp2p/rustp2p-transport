use clap::Parser;
use env_logger::Env;
use rustp2p::cipher::Algorithm;
use rustp2p::config::{PipeConfig, TcpPipeConfig, UdpPipeConfig};
use rustp2p::pipe::PeerNodeAddress;
use rustp2p::protocol::node_id::GroupCode;
use std::io;
use std::net::Ipv4Addr;
use std::time::Duration;
use tcp_ip::ip_stack::IpStackConfig;
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
    /// Nodes with the same group_code can form a network
    #[arg(short, long)]
    group_code: String,
    /// Connect node IP
    /// example: --connect 10.26.0.3
    #[arg(short, long)]
    connect: Option<Ipv4Addr>,
    /// Listen local port
    #[arg(short = 'P', long, default_value = "23333")]
    port: u16,
}
/// Node A: ./connect -- --ip 10.26.0.2 -g 123
/// Node B: ./connect -- --ip 10.26.0.3 -g 123 -c 10.26.0.2 --peer tcp://127.0.0.1:23333
///
///
#[tokio::main]
async fn main() -> io::Result<()> {
    main0().await
}

pub async fn main0() -> io::Result<()> {
    let Args {
        peer,
        ip,
        group_code,
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

    let udp_config = UdpPipeConfig::default().set_udp_ports(vec![port]);
    let tcp_config = TcpPipeConfig::default().set_tcp_port(port);
    let p2p_config = PipeConfig::empty()
        .set_udp_pipe_config(udp_config)
        .set_tcp_pipe_config(tcp_config)
        .set_direct_addrs(addrs)
        .set_group_code(string_to_group_code(&group_code))
        .set_encryption(Algorithm::AesGcm("password".to_string()))
        .set_node_id(ip.into());

    let ip_stack_config = IpStackConfig::default();
    let ip_stack = rustp2p_transport::transport(p2p_config, ip_stack_config).await?;
    if let Some(connect) = connect {
        log::info!("***** connect stream {connect}");
        let mut tcp_stream = tcp_ip::tcp::TcpStream::connect(
            ip_stack,
            format!("{ip}:12345").parse().unwrap(),
            format!("{connect}:12345").parse().unwrap(),
        )
        .await?;
        let mut reader = BufReader::new(tokio::io::stdin());
        let mut string = String::new();
        loop {
            string.clear();
            print!("Please enter the char:");
            reader.read_line(&mut string).await?;
            println!("input: {}", string);
            tcp_stream.write_all(string.as_bytes()).await?;
        }
    } else {
        let mut tcp_listener =
            tcp_ip::tcp::TcpListener::bind(ip_stack.clone(), "0.0.0.0:12345".parse().unwrap())
                .await?;
        log::info!("***** tcp_listener accept");
        loop {
            let (mut stream, addr) = tcp_listener.accept().await?;
            log::info!("***** accept stream {addr}");
            tokio::spawn(async move {
                let mut buf = vec![0; 65536];
                loop {
                    match stream.read(&mut buf).await {
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
fn string_to_group_code(input: &str) -> GroupCode {
    let mut array = [0u8; 16];
    let bytes = input.as_bytes();
    let len = bytes.len().min(16);
    array[..len].copy_from_slice(&bytes[..len]);
    array.into()
}

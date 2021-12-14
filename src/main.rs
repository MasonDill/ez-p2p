#![warn(clippy::all)]
// #![allow(unused)]

use std::fs;
use std::str::FromStr;
use std::{
    net::SocketAddr,
    path::{Path, PathBuf},
    time::Duration,
};

use bytes::Bytes;
use color_eyre::eyre::Result;
use local_ip_address::local_ip;

use qp2p::{Config, Endpoint, IncomingConnections};
use structopt::StructOpt;
use tokio::spawn;

#[derive(Debug, StructOpt)]
#[structopt(about = "Send or receive files. Receiving is default unless the send flag is used.")]
struct Opt {
    /// Specifies which file to send. (Default is receiving if this flag is unused.)
    #[structopt(short, long, name = "FILE", parse(from_os_str))]
    send: Option<PathBuf>,

    /// Peer's network address (ip:port)
    #[structopt(short = "o", long="ip", parse(try_from_str = parse_socket_addr))]
    peer_address: Option<SocketAddr>,
}

fn parse_socket_addr(src: &str) -> Result<SocketAddr, std::net::AddrParseError> {
    SocketAddr::from_str(src)
}

#[tokio::main]
async fn main() -> Result<()> {
    // tracing_subscriber::fmt()
    //     .with_max_level(Level::TRACE)
    //     .init();
    color_eyre::install()?;
    let opt = Opt::from_args();

    if let Some(file) = opt.send {
        // sending
        if Path::exists(&file) {
            let tx = spawn(send(opt.peer_address.unwrap(), fs::read(file)?));
            tx.await??;
        } else {
            panic!("File does not exist.")
        }
    } else {
        // receiving
        let rx = spawn(receive());
        rx.await??;
    }

    Ok(())
}

async fn send(peer_addr: SocketAddr, file: Vec<u8>) -> Result<()> {
    let (node, _incoming_connections) = make_node().await?;

    let msg = Bytes::from(file);
    // println!("Sending to {:?} --> {:?}\n", peer_address, msg);
    println!("Sending to {:?}", peer_addr);
    let (connection, _connection_incoming) = node.connect_to(&peer_addr).await?;
    connection.send(msg).await?;
    connection.close();

    Ok(())
}

async fn receive() -> Result<()> {
    println!("Waiting for connection...");

    let (_node, mut incoming_connections) = make_node().await?;

    while let Some((connection, mut incoming_messages)) = incoming_connections.next().await {
        println!("Connection made to {:?}", connection.remote_address());

        // loop over incoming messages
        while let Some(bytes) = incoming_messages.next().await? {
            // println!("Message received from {:?} --> {:?}", src, bytes);
            println!("File received; writing to output.bin.");
            fs::write("output.bin", bytes)?;
            println!("Done!");
        }
    }

    Ok(())
}

async fn make_node() -> Result<(Endpoint, IncomingConnections)> {
    let internal_ip = local_ip().expect("Couldn't get internal IP.");
    let port = port_scanner::request_open_port().expect("Unable to find an available port.");
    let external_ip = public_ip::addr().await;

    let local_addr = SocketAddr::new(internal_ip, port);

    let printed_addr = {
        if let Some(external_ip) = external_ip {
            SocketAddr::new(external_ip, port)
        } else {
            local_addr
        }
    };
    println!("Endpoint created at {:?}", printed_addr);

    let (node, incoming_connections, _) = Endpoint::new(
        local_addr,
        &[],
        Config {
            forward_port: true,
            idle_timeout: Some(Duration::from_secs(120)),
            ..Default::default()
        },
    )
    .await?;

    Ok((node, incoming_connections))
}

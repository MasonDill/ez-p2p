#![warn(clippy::all)]
// #![allow(unused)]

use std::fs;
use std::str::FromStr;
use std::{
    net::{Ipv4Addr, SocketAddr},
    path::{Path, PathBuf},
    time::Duration,
};

use bytes::Bytes;
use color_eyre::eyre::Result;
use qp2p::{Config, Endpoint, IncomingConnections};
use structopt::StructOpt;

use tokio::spawn;

#[derive(Debug, StructOpt)]
#[structopt(about = "Send or receive files. Receiving is default unless the send flag is used.")]
struct Opt {
    /// Specifies which file to send. (Default is receiving if this flag is unused.)
    #[structopt(short, long, name = "FILE", parse(from_os_str))]
    send: Option<PathBuf>,

    /// Network port
    #[structopt(short, long)]
    port: u16,

    /// Network port
    #[structopt(short = "o", long, parse(try_from_str = parse_socket_addr))]
    peer_address: Option<SocketAddr>,
}

fn parse_socket_addr(src: &str) -> Result<SocketAddr, std::net::AddrParseError> {
    SocketAddr::from_str(src)
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let opt = Opt::from_args();

    if let Some(file) = opt.send {
        if Path::exists(&file) {
            let tx = spawn(send(opt.port, opt.peer_address.unwrap(), fs::read(file)?));
            tx.await??;
        } else {
            panic!("File does not exist.")
        }
    } else {
        let rx = spawn(receive(opt.port));
        rx.await??;
    }

    Ok(())
}

async fn send(port: u16, peer_address: SocketAddr, file: Vec<u8>) -> Result<()> {
    let (node, _incoming_connections) = make_node(port).await?;

    let msg = Bytes::from(file);
    // println!("Sending to {:?} --> {:?}\n", peer_address, msg);
    println!("Sending to {:?}", peer_address);
    let (connection, _connection_incoming) = node.connect_to(&peer_address).await?;
    connection.send(msg).await?;
    connection.close();

    Ok(())
}

async fn receive(port: u16) -> Result<()> {
    println!("Waiting for connection...");

    let (_node, mut incoming_connections) = make_node(port).await?;

    while let Some((connection, mut incoming_messages)) = incoming_connections.next().await {
        let src = connection.remote_address();
        println!("Connection made to {:?}", src);

        // loop over incoming messages
        while let Some(bytes) = incoming_messages.next().await? {
            // println!("Message received from {:?} --> {:?}", src, bytes);
            println!("File received; writing to output.bin.");
            fs::write("output.bin", bytes);
            println!("Done!");
        }
    }

    Ok(())
}

async fn make_node(port: u16) -> Result<(Endpoint, IncomingConnections)> {
    let (node, incoming_connections, _) = Endpoint::new(
        SocketAddr::from((Ipv4Addr::new(0, 0, 0, 0), port)),
        &[],
        Config {
            // forward_port: true, // TODO: change to true when testing NAT hole punching
            idle_timeout: Some(Duration::from_secs(120)),
            ..Default::default()
        },
    )
    .await?;

    Ok((node, incoming_connections))
}

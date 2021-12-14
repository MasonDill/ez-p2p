#![warn(clippy::all)]
// #![allow(unused)]

use std::{thread::sleep, time::Duration};

use anyhow::Result;
use tokio::{
    io::AsyncWriteExt,
    net::{TcpListener, TcpStream},
    spawn,
};

const ADDR: &str = "127.0.0.1:8080";

#[tokio::main]
async fn main() -> Result<()> {
    let rx = spawn(receive(ADDR));
    let tx = spawn(send(ADDR));

    tx.await??;
    rx.await??;

    Ok(())
}

async fn send(addr: &str) -> Result<()> {
    let mut stream = TcpStream::connect(addr).await?;

    stream.write_all(b"hello world!").await?;

    Ok(())
}

async fn receive(addr: &str) -> Result<()> {
    let listener = TcpListener::bind(addr).await?;

    if let Ok((rx_stream, addr)) = listener.accept().await {
        println!(
            "Listener accepted!\nStream: {:#?}\nAddress: {:#?}",
            rx_stream, addr
        );
    }

    sleep(Duration::from_secs(3));

    Ok(())
}

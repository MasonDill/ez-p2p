use std::net::{SocketAddr, SocketAddrV4, Ipv4Addr};

use clap::{builder::PossibleValue, Parser};
use tokio::{io::AsyncReadExt, io::AsyncWriteExt, net::TcpListener, net::TcpStream, net::TcpSocket};
use serde_json::Value;
use igd::SearchOptions;
use std::time::Duration;

#[derive(Parser)]
struct Args {   
    #[arg(index(1), required(true), value_parser([
        PossibleValue::new("server").alias("tx"),
        PossibleValue::new("client").alias("rx")]))]
    role: String,

    #[arg(index(2), required(true))]
    file: Option<String>,

    #[arg(short('p'), long, required_if_eq("role", "server"))]
    port: Option<u16>,

    #[arg(short('s'), long, required_if_eq("role", "client"))]
    peer_socket: Option<String>,

    #[arg(
        short,
        long,
        value_parser([
        PossibleValue::new("public").alias("WWW"),
        PossibleValue::new("private").alias("LAN")]),
        default_value("public")
    )]
    visibility: String,
}

/// Runtime worker thread count
const WORKER_THREADS: usize = 1;

/// Main entry point that parses command line arguments and starts either server or client mode
fn main() {
    let args = Args::parse();

    match args.role.as_str() {
        "server" => {
            tokio::runtime::Builder::new_multi_thread()
                .worker_threads(WORKER_THREADS)
                .enable_all()
                .build()
                .unwrap()
                .block_on(receive(&args))
                .unwrap();
        }
        "client" => {
            tokio::runtime::Builder::new_multi_thread()
                .worker_threads(WORKER_THREADS)
                .enable_all()
                .build()
                .unwrap()
                .block_on(trasmit(&args))
                .unwrap();
        }
        _ => {
            panic!("Invalid role");
        }
    }
}

/// Reads contents of a file asynchronously into a byte vector
/// 
/// # Arguments
/// * `file` - Path to the file to read
/// 
/// # Returns
/// * `Result<Vec<u8>>` - Byte vector containing file contents or error
async fn read_file(file : &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut file = tokio::fs::File::open(file).await?;
    let mut contents = vec![];
    file.read_to_end(&mut contents).await?;
    Ok(contents)
}

/// Writes byte contents to a file asynchronously
/// 
/// # Arguments
/// * `file` - Path to the file to write
/// * `contents` - Byte vector to write to file
/// 
/// # Returns
/// * `Result<()>` - Success or error status
async fn write_file(file : &str, contents : Vec<u8>) -> Result<(), Box<dyn std::error::Error>> {
    let mut file = tokio::fs::File::create(file).await?;
    file.write_all(&contents).await?;
    Ok(())
}

/// Transmits file contents to a remote TCP server
/// 
/// # Arguments
/// * `args` - Command line arguments containing peer socket and file path
/// 
/// # Returns
/// * `Result<()>` - Success or error status
async fn trasmit(args : &Args) -> Result<(), Box<dyn std::error::Error>> {
    let peer_address: &Option<String> = &args.peer_socket;
    let mut stream = TcpStream::connect(args.peer_socket.as_ref().unwrap()).await?;
    
    // Load the data from the file
    let mut contents = read_file(args.file.as_ref().unwrap()).await?;
    stream.write_all(&contents).await?;

    Ok(())
}

/// Downloads and prints data from a TCP stream
/// 
/// # Arguments
/// * `stream` - TCP stream to read from
/// 
/// # Returns
/// * `Result<()>` - Success or error status
async fn download_from_tcp_stream(mut stream: TcpStream) -> Result<(), Box<dyn std::error::Error>> {
    let mut buf = [0; 1024];
    let n = stream.read(&mut buf).await?;
    println!("{}", std::str::from_utf8(&buf[..n]).unwrap());
    Ok(())
}

/// Local host binding address
const LOCAL_HOST: &str = "0.0.0.0";

/// Adds a UPnP port mapping
/// 
/// # Arguments
/// * `port` - Port to forward
/// 
/// # Returns
/// * `Result<()>` - Success or error status
async fn add_port_mapping(port: u16) -> Result<(), Box<dyn std::error::Error>> {
    let gateway = igd::search_gateway(SearchOptions {
        timeout: Some(Duration::from_secs(5)),
        ..Default::default()
    })?;

    let private_ip: Ipv4Addr = fetch_private_ip().await?.parse()?;
    let local_addr = SocketAddrV4::new(private_ip, port);

    gateway.add_port(
        igd::PortMappingProtocol::TCP,
        port,
        local_addr,
        60 * 30, // 30 minutes lease
        "rust-file-transfer",
    )?;

    Ok(())
}

/// Removes a UPnP port mapping
/// 
/// # Arguments
/// * `port` - Port to unmap
/// 
/// # Returns
/// * `Result<()>` - Success or error status
async fn remove_port_mapping(port: u16) -> Result<(), Box<dyn std::error::Error>> {
    let gateway = igd::search_gateway(SearchOptions {
        timeout: Some(Duration::from_secs(5)),
        ..Default::default()
    })?;

    gateway.remove_port(
        igd::PortMappingProtocol::TCP,
        port,
    )?;

    Ok(())
}

/// Starts TCP server to receive incoming file transfers
/// 
/// # Arguments
/// * `args` - Command line arguments containing port and visibility settings
/// 
/// # Returns
/// * `Result<()>` - Success or error status
async fn receive(args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    let port = args.port.unwrap();
    let private_socket_address = format!("{}:{}", LOCAL_HOST, port);
    let listener = TcpListener::bind(&private_socket_address)
        .await
        .unwrap_or_else(|e| {
            eprintln!("Failed to bind to port {} - {}", port, e);
            std::process::exit(1);
        });
    
    let public_ip = match args.visibility.as_str() {
        "public" => {
            // Set up UPnP port forwarding for public visibility
            if let Err(e) = add_port_mapping(port).await {
                eprintln!("Warning: Failed to set up UPnP port forwarding: {}", e);
            }
            fetch_public_ip().await?
        }
        "private" => fetch_private_ip().await?,
        _ => unreachable!(),
    };
    
    let public_socket_address = format!("{}:{}", public_ip, port);
    println!("Listening on socket {}", &public_socket_address);

    let (socket, _) = listener.accept().await?;
    println!("Accepted connection from {}", socket.peer_addr()?);
    
    tokio::spawn(async move {
        download_from_tcp_stream(socket).await.unwrap();
    }).await?;

    // Clean up UPnP port mapping if it was public
    if args.visibility == "public" {
        if let Err(e) = remove_port_mapping(port).await {
            eprintln!("Warning: Failed to remove UPnP port mapping: {}", e);
        }
    }

    println!("Transmission complete!");
    Ok(())
}

/// Fetches the local private IP address by establishing a connection to a public DNS server
/// 
/// # Returns
/// * `Result<String>` - Private IP address or error
async fn fetch_private_ip() -> Result<String, Box<dyn std::error::Error>> {
    // Connect to a public address to establish a TCP connection
    let stream = TcpStream::connect("8.8.8.8:443").await?;
    let ip = stream.local_addr()?;
    
    Ok(ip.ip().to_string())
}

/// Fetches the public IP address using ipify.org API
/// 
/// # Returns
/// * `Result<String>` - Public IP address or error
async fn fetch_public_ip() -> Result<String, Box<dyn std::error::Error>> {
    let mut stream = TcpStream::connect("api.ipify.org:80").await?;
    stream.write_all(b"GET /?format=json HTTP/1.1\r\nHost: api.ipify.org\r\n\r\n").await?;
    let mut buf = [0; 1024];
    let n = stream.read(&mut buf).await?;
    let response = std::str::from_utf8(&buf[..n]).unwrap().lines().last().unwrap();

    let json: Value = serde_json::from_str(response)?;
    let ip: String = json["ip"].as_str().unwrap().to_string();
    return Ok(ip);
}
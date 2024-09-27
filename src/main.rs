use clap::{builder::PossibleValue, Parser};
use tokio::{io::AsyncReadExt, io::AsyncWriteExt, net::TcpListener, net::TcpStream};
use serde_json::Value;

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

const WORKER_THREADS: usize = 1;
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

async fn read_file(file : &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut file = tokio::fs::File::open(file).await?;
    let mut contents = vec![];
    file.read_to_end(&mut contents).await?;
    Ok(contents)
}

async fn write_file(file : &str, contents : Vec<u8>) -> Result<(), Box<dyn std::error::Error>> {
    let mut file = tokio::fs::File::create(file).await?;
    file.write_all(&contents).await?;
    Ok(())
}

async fn trasmit(args : &Args) -> Result<(), Box<dyn std::error::Error>> {
    let peer_address: &Option<String> = &args.peer_socket;
    let mut stream = TcpStream::connect(args.peer_socket.as_ref().unwrap()).await?;
    
    // Load the data from the file
    let mut contents = read_file(args.file.as_ref().unwrap()).await?;
    stream.write_all(&contents).await?;

    Ok(())
}

async fn download_from_tcp_stream(mut stream: TcpStream) -> Result<(), Box<dyn std::error::Error>> {
    let mut buf = [0; 1024];
    let n = stream.read(&mut buf).await?;
    println!("{}", std::str::from_utf8(&buf[..n]).unwrap());
    Ok(())
}

const LOCAL_HOST: &str = "0.0.0.0";
async fn receive(args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    let private_socket_address = format!("{}:{}", LOCAL_HOST, args.port.unwrap());
    let listener = TcpListener::bind(&private_socket_address)
        .await
        .unwrap_or_else(|e| {
            eprintln!("Failed to bind to port {} - {}", args.port.unwrap(), e);
            std::process::exit(1); // Exit if binding fails
        });
    
    let public_ip: &str = match args.visibility.as_str() {
        "public" => &fetch_public_ip().await?,
        "private" => &fetch_private_ip().await?,
        _ => unreachable!(),
    };
    let public_socket_address = format!("{}:{}", public_ip, args.port.unwrap());
    println!("Listening on socket {}", &public_socket_address);

    let (socket, _) = listener.accept().await?;
    println!("Accepted connection from {}", socket.peer_addr()?);
    tokio::spawn(async move {
        download_from_tcp_stream(socket).await.unwrap();
    }).await?;

    println!("Transmission complete!");
    Ok(())
}

async fn fetch_private_ip() -> Result<String, Box<dyn std::error::Error>> {
    let output = std::process::Command::new("ipconfig")
        .output()
        .expect("Failed to execute ipconfig");
    let output = String::from_utf8(output.stdout).unwrap();
    let ip = output
        .lines()
        .find(|line| line.contains("IPv4 Address"))
        .unwrap()
        .split(':')
        .last()
        .unwrap()
        .trim();

    return Ok(ip.to_string());
}

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
use clap::error;
use clap::{builder::PossibleValue, Parser};
use tokio::{io::AsyncReadExt, io::AsyncWriteExt, net::TcpListener, net::TcpSocket, net::TcpStream};
use std::net::SocketAddr;
use std::io;

#[derive(Parser)]
struct Args {   
    #[arg(index(1), required(true), value_parser([
        PossibleValue::new("server").alias("tx"),
        PossibleValue::new("client").alias("rx")]))]
    role: String,

    #[arg(short, long)]
    file: Option<String>,

    #[arg(short('p'), long, required_if_eq("role", "server"))]
    port: Option<u16>,

    #[arg(short('a'), long, required_if_eq("role", "client"))]
    peer_address: Option<String>,
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
    let peer_address: &Option<String> = &args.peer_address;
    let mut stream = TcpStream::connect(args.peer_address.as_ref().unwrap()).await?;
    
    // Load the data from the file
    let mut contents = read_file(args.file.as_ref().unwrap()).await?;
    stream.write_all(&contents).await?;

    Ok(())
}

async fn receive(args : &Args) -> io::Result<()> {
    let socket_address = format!("127.0.0.1:{}", args.port.unwrap());
    let listener = TcpListener::bind(&socket_address).await?;
    println!("Listening on: {}", listener.local_addr()?);

    //let file: &str = args.file.as_ref().unwrap();
    loop{
        let (mut socket, _) = listener.accept().await?;
        println!("Accepted connection from {}", socket.peer_addr()?);
        tokio::spawn(async move {
            let mut buf = [0; 1024];
            let n = socket.read(&mut buf).await.unwrap();
            
            // write the data to the file
            //write_file(&file, buf.to_vec());
            print!("{}", std::str::from_utf8(&buf[..n]).unwrap());
        });
    }
    
}

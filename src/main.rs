use clap::{builder::PossibleValue, Parser};

#[derive(Parser)]
struct Args {   
    #[arg(short, long, required(true), value_parser([
        PossibleValue::new("server").alias("tx"),
        PossibleValue::new("client").alias("rx")]))]
    role: String,

    #[arg(short, long, required_if_eq("role", "server"))]
    file: Option<String>,

    #[arg(short, long, required_if_eq("role", "client"))]
    peer_address: Option<String>,
}
fn main() {
    let args = Args::parse();
}

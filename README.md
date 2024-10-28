# ez-p2p
ez-p2p is a simple CLI-based p2p filesharing application written in Rust.

## Installation
1. Clone the repo:
   ```bash
   git clone https://github.com/username/project-name.git](https://github.com/MasonDill/ez-p2p.git
   # or
   git clone git@github.com:MasonDill/ez-p2p.git
   ```
2. Build and run:
   ```bash
   cargo build --release
   ```

## Usage
  ```bash
  Usage: ./target/release/ez-p2p.exe [OPTIONS] <ROLE> <FILE>
  
  Arguments:
    <ROLE>  [possible values: server, client]
    <FILE>
  
  Options:
    -p, --port <PORT>
    -s, --peer-socket <PEER_SOCKET>
    -v, --visibility <VISIBILITY>    [default: public] [possible values: public, private]
    -h, --help                       Print help
  ```
### Examples
#### Server (transmitting)
  ```bash
  ./target/release/ez-p2p server -p 8080 -v public test.txt
  ```
#### Client (recieving)
  ```bash
  ./target/release/ez-p2p client -s peer/address:port test.txt
  ```

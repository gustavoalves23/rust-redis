use std::net::SocketAddr;

use tokio::{io::AsyncWriteExt, net::TcpListener};

const PING_RESPONDE: [u8; 7] = *b"+PONG\r\n";

#[tokio::main]
async fn main() {
    let addr = "127.0.0.1:6379";
    let socket_addr = addr.parse::<SocketAddr>().unwrap();

    let listener = TcpListener::bind(socket_addr).await.unwrap();
    loop {
        match listener.accept().await {
            Ok((mut stream, _)) => {
                stream.write_all(&PING_RESPONDE).await.unwrap();
            }
            Err(e) => {
                eprintln!("Error: {}", e);
            }
        }
    }
}

use std::net::SocketAddr;

use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    let addr = "127.0.0.1:6379";
    let socket_addr = addr.parse::<SocketAddr>().unwrap();

    let listener = TcpListener::bind(socket_addr).await.unwrap();
    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                println!("Accepted new connection!, Stream: {:?}", stream);
            }
            Err(e) => {
                eprintln!("Error: {}", e);
            }
        }
    }
}

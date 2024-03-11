use std::{net::SocketAddr, str::from_utf8};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

const PING_RESPONSE: [u8; 7] = *b"+PONG\r\n";

enum RedisCommands {
    Ping,
}

impl ToString for RedisCommands {
    fn to_string(&self) -> String {
        match self {
            RedisCommands::Ping => "ping\r\n".to_owned(),
        }
    }
}

#[tokio::main]
async fn main() {
    let addr = "127.0.0.1:6379";
    let socket_addr = addr.parse::<SocketAddr>().unwrap();

    let listener = TcpListener::bind(socket_addr).await.unwrap();
    loop {
        match listener.accept().await {
            Ok((mut stream, _)) => loop {
                handle_client(&mut stream).await;
            },
            Err(e) => {
                eprintln!("Error: {}", e);
            }
        }
    }
}

async fn handle_client(stream: &mut TcpStream) {
    let mut buf = [0; 1024];
    stream.read(&mut buf).await.unwrap();
    let cmd = from_utf8(&buf).unwrap();
    println!("{}", cmd);
    match cmd {
        c if c.contains(&RedisCommands::Ping.to_string()) => {
            stream.write_all(&PING_RESPONSE).await.unwrap();
        }
        _ => unimplemented!(),
    }
}

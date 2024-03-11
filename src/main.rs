use std::{net::SocketAddr, str::from_utf8};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

const PING_RESPONSE: [u8; 7] = *b"+PONG\r\n";

enum RedisCommands {
    Ping,
}

impl RedisCommands {
    fn as_command(&self) -> String {
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
        if let Ok((stream, _)) = listener.accept().await {
            tokio::spawn(handle_client(stream));
        } else {
            panic!()
        }
    }
}

async fn handle_client(mut stream: TcpStream) {
    loop {
        let mut buf = [0; 1024];
        stream.read(&mut buf).await.unwrap();
        let cmd = from_utf8(&buf).unwrap();
        match cmd {
            c if c.contains(&RedisCommands::Ping.as_command()) => {
                stream.write_all(&PING_RESPONSE).await.unwrap();
            }
            _ => unimplemented!(),
        }
    }
}

use std::{convert::Infallible, fmt::Error, net::SocketAddr, str::from_utf8};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

const PING_RESPONSE: [u8; 7] = *b"+PONG\r\n";

#[derive(Debug)]
enum RedisCommands {
    Ping,
    Echo,
}

impl RedisCommands {
    fn as_command(&self) -> &str {
        match self {
            RedisCommands::Ping => "PING",
            RedisCommands::Echo => "ECHO",
        }
    }
}

#[tokio::main]
async fn main() {
    let addr = "127.0.0.1:6379";
    let socket_addr = addr.parse::<SocketAddr>().unwrap();

    let listener = TcpListener::bind(socket_addr).await.unwrap();

    loop {
        if let Ok((mut stream, _)) = listener.accept().await {
            tokio::spawn(async move {
                loop {
                    handle_client(&mut stream).await
                }
            });
        } else {
            panic!()
        }
    }
}

fn parse_redis_command<'a>(
    raw_command: &'a str,
) -> Result<(RedisCommands, Vec<&'a str>, i32), Error> {
    match raw_command.strip_prefix("*") {
        Some(cmd) => {
            let mut command_arr = cmd.split("\r\n").into_iter();
            let count: i32 = command_arr.next().unwrap().parse::<i32>().unwrap();
            command_arr.next();

            let command = match command_arr.next().unwrap().to_uppercase().trim() {
                cmd if cmd == RedisCommands::Ping.as_command() => RedisCommands::Ping,
                cmd if cmd == RedisCommands::Echo.as_command() => RedisCommands::Echo,
                _cmd => panic!(),
            };
            let args: Vec<_> = command_arr
                .filter(|x| x != &"" && !x.contains("$"))
                .collect();

            Ok::<_, _>((command, args, count))
        }
        None => Err(Error),
    }
}

async fn handle_ping_command(stream: &mut TcpStream) -> Result<(), Infallible> {
    stream.write_all(&PING_RESPONSE).await.unwrap();
    Ok(())
}

async fn handle_echo_command(stream: &mut TcpStream, args: Vec<&str>) -> Result<(), Error> {
    if args.len() != 1 {
        return Err(Error);
    }

    let msg = args.iter().next().unwrap();
    let msg_len = msg.len();
    let res = format!("${msg_len}\r\n{msg}\r\n");

    stream.write_all(res.as_bytes()).await.unwrap();
    Ok(())
}

async fn handle_client(stream: &mut TcpStream) {
    let mut buf = [0; 1024];
    let range = stream.read(&mut buf).await.unwrap();
    let cmd = from_utf8(&buf[0..range]).unwrap();
    if let Ok(data) = parse_redis_command(cmd) {
        match data {
            (RedisCommands::Ping, _, _) => {
                handle_ping_command(stream).await.unwrap();
            }
            (RedisCommands::Echo, args, _) => {
                handle_echo_command(stream, args).await.unwrap();
            }
        }
    }
}

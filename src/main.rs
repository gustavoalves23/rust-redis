use std::{
    collections::HashMap,
    convert::Infallible,
    fmt::Error,
    net::SocketAddr,
    str::from_utf8,
    sync::{Arc, Mutex},
};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

#[derive(Debug)]
enum RedisCommands {
    Ping,
    Echo,
    Set,
    Get,
}

impl RedisCommands {
    fn as_command(&self) -> &str {
        match self {
            RedisCommands::Ping => "PING",
            RedisCommands::Echo => "ECHO",
            RedisCommands::Set => "SET",
            RedisCommands::Get => "GET",
        }
    }
}

#[tokio::main]
async fn main() {
    let addr = "127.0.0.1:6379";
    let socket_addr = addr.parse::<SocketAddr>().unwrap();

    let listener = TcpListener::bind(socket_addr).await.unwrap();

    let storage: Arc<Mutex<HashMap<String, String>>> =
        Arc::new(Mutex::new(HashMap::<String, String>::new()));
    loop {
        if let Ok((mut stream, _)) = listener.accept().await {
            let storage = Arc::clone(&storage);
            tokio::spawn(async move {
                loop {
                    let storage = Arc::clone(&storage);
                    handle_client(&mut stream, storage).await
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
                cmd if cmd == RedisCommands::Set.as_command() => RedisCommands::Set,
                cmd if cmd == RedisCommands::Get.as_command() => RedisCommands::Get,
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

fn write_to_storage(storage: Arc<Mutex<HashMap<String, String>>>, val: (String, String)) {
    let mut storage = storage.lock().unwrap();
    let (k, v) = val;
    storage.insert(k, v);
}
fn read_from_storage(storage: Arc<Mutex<HashMap<String, String>>>, key: String) -> String {
    let storage = storage.lock().unwrap();
    storage.get(&key).unwrap().to_string()
}

async fn handle_ping_command(stream: &mut TcpStream) -> Result<(), Infallible> {
    const PING_RESPONSE: [u8; 7] = *b"+PONG\r\n";
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

async fn handle_set_command(
    stream: &mut TcpStream,
    args: Vec<&str>,
    storage: Arc<Mutex<HashMap<String, String>>>,
) -> Result<(), Infallible> {
    let mut args = args.iter();
    let key = args.next().unwrap();
    let val = args.next().unwrap();

    write_to_storage(storage, (key.to_string(), val.to_string()));

    stream.write_all(b"+OK\r\n").await.unwrap();
    Ok(())
}

async fn handle_get_command(
    stream: &mut TcpStream,
    args: Vec<&str>,
    storage: Arc<Mutex<HashMap<String, String>>>,
) -> Result<(), Error> {
    if args.len() != 1 {
        return Err(Error);
    }

    let key = args.iter().next().unwrap();
    let msg = read_from_storage(storage, key.to_string());
    let msg_len = msg.len();
    let res = format!("${msg_len}\r\n{msg}\r\n");

    stream.write_all(res.as_bytes()).await.unwrap();
    Ok(())
}

async fn handle_client(stream: &mut TcpStream, storage: Arc<Mutex<HashMap<String, String>>>) {
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
            (RedisCommands::Set, args, _) => {
                handle_set_command(stream, args, storage).await.unwrap();
            }
            (RedisCommands::Get, args, _) => {
                handle_get_command(stream, args, storage).await.unwrap();
            }
        }
    }
}

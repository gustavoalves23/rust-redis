use std::{
    collections::{vec_deque::VecDeque, HashMap},
    convert::Infallible,
    env,
    fmt::Error,
    net::SocketAddr,
    str::from_utf8,
    sync::{Arc, Mutex},
    time::SystemTime,
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
    Config,
}

impl RedisCommands {
    fn as_command(&self) -> &str {
        match self {
            RedisCommands::Ping => "PING",
            RedisCommands::Echo => "ECHO",
            RedisCommands::Set => "SET",
            RedisCommands::Get => "GET",
            RedisCommands::Config => "CONFIG",
        }
    }
}
type Storage = Arc<Mutex<HashMap<String, RedisEntry>>>;

#[derive(Debug)]
struct RedisEntry {
    val: String,
    ttl: Option<u32>,
    created_at: SystemTime,
}

type ProgramArgs = HashMap<String, String>;

fn get_args() -> ProgramArgs {
    let mut args = HashMap::<String, String>::new();
    let mut args_vec: VecDeque<String> = env::args().collect();
    VecDeque::pop_front(&mut args_vec).unwrap();

    if args_vec.len() % 2 != 0 {
        panic!()
    }
    for (index, key) in args_vec.iter().enumerate().step_by(2) {
        let val = &args_vec[index + 1];
        let (_, key) = key.split_at(2);
        args.insert(key.to_string(), val.to_string());
    }

    args
}

#[tokio::main]
async fn main() {
    let addr = "127.0.0.1:6379";
    let socket_addr = addr.parse::<SocketAddr>().unwrap();

    let listener = TcpListener::bind(socket_addr).await.unwrap();

    let storage: Storage = Arc::new(Mutex::new(HashMap::<String, RedisEntry>::new()));
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
            unimplemented!()
        }
    }
}
fn bulk_response(cmd: Vec<&str>) -> String {
    let new_line = "\r\n";
    let mut res = String::from("*");
    let qty = cmd.len().to_string();
    res.push_str(&qty);
    res.push_str(&new_line);
    cmd.iter().for_each(|str| {
        res.push_str(&format!("${}", str.len()));
        res.push_str(&new_line);
        res.push_str(&str);
        res.push_str(&new_line);
    });
    res
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
                cmd if cmd == RedisCommands::Config.as_command() => RedisCommands::Config,
                cmd => {
                    println!("Uninplemented command: {cmd}");
                    panic!()
                }
            };
            let args: Vec<_> = command_arr
                .filter(|x| x != &"" && !x.contains("$"))
                .collect();

            Ok::<_, _>((command, args, count))
        }
        None => Err(Error),
    }
}

fn write_to_storage(storage: Storage, val: (String, String, Option<u32>)) {
    let mut storage = storage.lock().unwrap();
    let (k, val, ttl) = val;
    let entry = RedisEntry {
        val,
        ttl,
        created_at: SystemTime::now(),
    };
    storage.insert(k, entry);
}
fn read_from_storage(storage: Storage, key: String) -> Option<String> {
    let storage = storage.lock().unwrap();
    let entry = storage.get(&key).unwrap();
    if let Some(ttl) = entry.ttl {
        if entry.created_at.elapsed().unwrap().as_millis() > ttl.into() {
            return None;
        }
    }
    Some(entry.val.to_string())
}

async fn handle_ping_command(stream: &mut TcpStream) -> Result<(), Infallible> {
    const PING_RESPONSE: [u8; 7] = *b"+PONG\r\n";
    stream.write_all(&PING_RESPONSE).await.unwrap();
    Ok(())
}

async fn handle_config_command(stream: &mut TcpStream, args: Vec<&str>) -> Result<(), Error> {
    if args.len() != 2
        || args[0].to_uppercase() != "GET"
        || ["--dir", "--dbfilename"].iter().any(|el| &args[1] == el)
    {
        dbg!("Invalid args");
        return Err(Error);
    }
    let requested_arg = args[1];
    let args = get_args();
    if let Some(arg_val) = args.get(requested_arg) {
        let cmd = [requested_arg, arg_val];
        let res = bulk_response(cmd.into());
        stream.write_all(res.as_bytes()).await.unwrap();
    };
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
    storage: Storage,
) -> Result<(), Error> {
    let mut args_iter = args.iter();
    let key = args_iter.next().unwrap();
    let val = args_iter.next().unwrap();
    let mut ttl: Option<u32> = None;

    if args.len() > 2 {
        let ttl_label = args_iter.next().unwrap();
        if ttl_label != &"px" {
            return Err(Error);
        }

        let ttl_agr = args_iter.next().unwrap().parse::<u32>().unwrap();
        ttl = Some(ttl_agr);
    }

    write_to_storage(storage, (key.to_string(), val.to_string(), ttl));

    stream.write_all(b"+OK\r\n").await.unwrap();
    Ok(())
}

async fn handle_get_command(
    stream: &mut TcpStream,
    args: Vec<&str>,
    storage: Storage,
) -> Result<(), Error> {
    if args.len() != 1 {
        return Err(Error);
    }

    let key = args.iter().next().unwrap();
    if let Some(msg) = read_from_storage(storage, key.to_string()) {
        println!("caiu 2");
        let msg_len = msg.len();
        let res = format!("${msg_len}\r\n{msg}\r\n");
        stream.write_all(res.as_bytes()).await.unwrap();
        Ok(())
    } else {
        stream.write_all(b"$-1\r\n").await.unwrap();
        Ok(())
    }
}

async fn handle_client(stream: &mut TcpStream, storage: Storage) {
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
            (RedisCommands::Config, args, _) => {
                handle_config_command(stream, args).await.unwrap();
            }
        }
    }
}

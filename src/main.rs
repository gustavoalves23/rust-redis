use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:6379").await.unwrap();

    match listener.accept().await {
        Ok(_socket) => {
            println!("Accepted new connection!");
        }
        Err(e) => {
            eprintln!("Error: {}", e);
        }
    }
}

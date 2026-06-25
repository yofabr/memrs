use color_eyre::Result;
use tokio::net::TcpListener;


#[derive(Debug, Clone)]
pub enum ClientType {
    CLI,
    APPLICATION,
}


#[derive(Debug, Clone)]
pub struct Client {
    is_authenticated: bool,
    connected_since: String,
    clientType: ClientType,
}

pub async fn init_listener() -> Result<()> {
    let listener = TcpListener::bind("0.0.0.0:8080").await?;
    loop {
        let (_stream, _) = listener.accept().await?;
    }
}

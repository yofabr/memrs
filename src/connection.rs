use color_eyre::Result;
use tokio::net::TcpListener;

use crate::core::CONFIG;


#[derive(Debug, Clone)]
pub enum ClientType {
    CLI,
    APPLICATION,
}


#[derive(Debug, Clone)]
pub struct Client {
    is_authenticated: bool,
    // connected_since: String,
    // clientType: ClientType,
}

pub async fn init_listener() -> Result<()> {
    let port_number = CONFIG.lock().unwrap().port;
    let address = format!("0.0.0.0:{}", port_number);
    let listener = TcpListener::bind(&address).await?;

    println!("[INFO] Started TcpListener at: {}", &address);
    loop {
        let (_stream, _) = listener.accept().await?;
    }
}

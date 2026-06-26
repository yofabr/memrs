use color_eyre::Result;
use memrs::connection::init_listener;


#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    init_listener().await?;
    println!("Hello from memrs!");
    Ok(())
}

use color_eyre::Result;
use memrs::config;


#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let conf = config::Config::load_config();
    conf.print_config();
    println!("Hello from memrs!");
    Ok(())
}

# memrs

An in-memory key-value data store written Rust. 

Memrs started as a learning project to explore systems programming with async Rust, 
but evolved into a functional lightweight key value pair database layer with Redis-compatible commands, 
connection pooling, TTL-based expiry, and a RESP-compatible wire protocol.

## Usage

### Server (Docker)

```bash
docker run -d -p 7898:7898 yofabr/memrs
```

### Client library

```toml
[dependencies]
memrs-rs = "0.1"
```

```rust
use memrs_rs::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = Client::connect("127.0.0.1:7898").await?;
    client.set("name", "memrs").await?;
    let val: String = client.get("name").await?;
    println!("{val}"); // "memrs"
    Ok(())
}
```

### CLI

```bash
cargo install memrs-cli
memrs-cli -a mypassword
memrs> SET foo bar
+OK
memrs> GET foo
bar
```

## License

MIT

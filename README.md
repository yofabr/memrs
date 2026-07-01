# memrs

Memrs is an in-memory key-value data store built from scratch in Rust. 
Designed as a lightweight Redis alternative focused on simplicity, minimal dependencies, and a clean codebase that's easy to understand and extend.

Memrs started as a learning project to explore systems programming with async Rust, 
but evolved into a functional caching layer with Redis-compatible commands, 
connection pooling, TTL-based expiry, and a RESP-compatible wire protocol.

demo video:
<a href="https://asciinema.org/a/1259657" target="_blank"><img src="https://asciinema.org/a/1259657.svg" /></a>

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

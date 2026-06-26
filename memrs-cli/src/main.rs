use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use std::env;
use std::io::Write;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

fn print_help() {
    println!("\x1b[1mConnection commands:\x1b[0m");
    println!("  \x1b[33mPING\x1b[0m                       → +PONG");
    println!("  \x1b[33mAUTH <password>\x1b[0m             → Authenticate with the server");
    println!("  \x1b[33mHELP\x1b[0m                        → Show this help message");
    println!("  \x1b[33mCLEAR\x1b[0m                       → Clear the terminal");
    println!("  \x1b[33mCONNECT <host> <port>\x1b[0m       → Connect to a different server");
    println!("  \x1b[33mEXIT\x1b[0m / \x1b[33mQUIT\x1b[0m               → Disconnect and exit");
    println!();
    println!("\x1b[1mData commands:\x1b[0m");
    println!("  \x1b[33mGET <key>\x1b[0m                   → Retrieve the value of a key");
    println!("  \x1b[33mSET <key> <value>\x1b[0m           → Insert or update a key-value pair");
    println!("  \x1b[33mEXISTS <key>\x1b[0m                → Check if a key exists");
    println!("  \x1b[33mDEL <key>\x1b[0m                   → Delete a key-value pair");
    println!();
    println!("\x1b[1mHash commands:\x1b[0m");
    println!("  \x1b[33mHSET <key> <field> <value>\x1b[0m  → Set a field in a hash");
    println!("  \x1b[33mHGET <key> <field>\x1b[0m          → Get a field from a hash");
    println!();
    println!("\x1b[1mList commands:\x1b[0m");
    println!("  \x1b[33mLPUSH <key> <value>\x1b[0m         → Prepend to a list");
    println!("  \x1b[33mRPUSH <key> <value>\x1b[0m         → Append to a list");
    println!("  \x1b[33mLPOP <key>\x1b[0m                  → Pop from the left");
    println!("  \x1b[33mRPOP <key>\x1b[0m                  → Pop from the right");
    println!();
    println!("\x1b[90m  -a, --auth <password>  authenticate on connect\x1b[0m");
    println!("\x1b[90m  -h, --host <host>      server host (default 127.0.0.1)\x1b[0m");
    println!("\x1b[90m  -p, --port <port>      server port (default 7898)\x1b[0m");
    println!();
    println!("\x1b[90mUse \x1b[0m\x1b[90m↑/↓\x1b[0m\x1b[90m for history.\x1b[0m");
}

fn format_response(raw: &str) -> String {
    if raw.starts_with('+') {
        format!("\x1b[32m{}\x1b[0m", raw)
    } else if raw.starts_with('-') {
        format!("\x1b[31m{}\x1b[0m", raw)
    } else if raw.starts_with(':') {
        format!("\x1b[36m{}\x1b[0m", raw)
    } else {
        format!("\x1b[37m{}\x1b[0m", raw)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let (mut host, mut port, mut password, cmd_args) = parse_args(&args);

    let (mut writer, mut buf_reader) = connect(&host, &port).await?;
    auto_auth(&mut writer, &mut buf_reader, &password).await;

    // --- one-shot mode ---
    if !cmd_args.is_empty() {
        for cmd in &cmd_args {
            writer.write_all(cmd.as_bytes()).await?;
            writer.write_all(b"\r\n").await?;
        }
        let mut line = String::new();
        buf_reader.read_line(&mut line).await?;
        print!("{}", line);
        return Ok(());
    }

    // --- interactive REPL mode ---
    let mut rl = DefaultEditor::new()?;
    let _ = rl.load_history(".memrs_history");

    println!("\x1b[1;32mmemrs-cli\x1b[0m connected to \x1b[1;36m{}:{}\x1b[0m", host, port);
    if !password.is_empty() {
        println!("\x1b[32mAuthenticated.\x1b[0m");
    }
    println!("\x1b[90mType HELP for available commands.\x1b[0m");

    loop {
        let prompt = format!("\x1b[1;32mmemrs\x1b[0m@\x1b[1;36m{}:{}\x1b[0m> ", host, port);

        let readline = rl.readline(&prompt);
        match readline {
            Ok(input) => {
                let trimmed = input.trim().to_string();
                if trimmed.is_empty() {
                    continue;
                }

                rl.add_history_entry(&trimmed)?;

                match trimmed.to_uppercase().as_str() {
                    "EXIT" | "QUIT" => {
                        println!("\x1b[33mGoodbye.\x1b[0m");
                        break;
                    }
                    "HELP" => {
                        print_help();
                        continue;
                    }
                    "CLEAR" => {
                        print!("\x1b[2J\x1b[1;1H");
                        let _ = std::io::stdout().flush();
                        continue;
                    }
                    _ => {}
                }

                if trimmed.starts_with("CONNECT ") {
                    let parts: Vec<&str> = trimmed.split_whitespace().collect();
                    if parts.len() >= 3 {
                        host = parts[1].to_string();
                        port = parts[2].to_string();
                        password = parts.get(3).map(|s| s.to_string()).unwrap_or_default();
                        match connect(&host, &port).await {
                            Ok((w, r)) => {
                                writer = w;
                                buf_reader = r;
                                auto_auth(&mut writer, &mut buf_reader, &password).await;
                                println!("\x1b[32mConnected to {}:{}\x1b[0m", host, port);
                            }
                            Err(e) => {
                                println!("\x1b[31mConnection failed: {}\x1b[0m", e)
                            }
                        }
                    } else {
                        println!("\x1b[31mUsage: CONNECT <host> <port> [password]\x1b[0m");
                    }
                    continue;
                }

                writer.write_all(trimmed.as_bytes()).await?;
                writer.write_all(b"\r\n").await?;

                let mut line = String::new();
                buf_reader.read_line(&mut line).await?;
                print!("{}\n", format_response(line.trim()));
            }
            Err(ReadlineError::Interrupted | ReadlineError::Eof) => {
                println!("\x1b[33mGoodbye.\x1b[0m");
                break;
            }
            Err(e) => {
                eprintln!("\x1b[31mError: {}\x1b[0m", e);
                break;
            }
        }
    }

    let _ = rl.append_history(".memrs_history");
    Ok(())
}

async fn auto_auth(
    writer: &mut tokio::net::tcp::OwnedWriteHalf,
    buf_reader: &mut BufReader<tokio::net::tcp::OwnedReadHalf>,
    password: &str,
) {
    writer.write_all(b"AUTH ").await.unwrap();
    writer.write_all(password.as_bytes()).await.unwrap();
    writer.write_all(b"\r\n").await.unwrap();
    let mut line = String::new();
    buf_reader.read_line(&mut line).await.unwrap();
    let line = line.trim();
    if line.starts_with('+') {
        if !password.is_empty() {
            println!("\x1b[32mAuthenticated.\x1b[0m");
        }
    } else if !password.is_empty() {
        println!("\x1b[31m{}\x1b[0m", line);
    }
}

async fn connect(host: &str, port: &str) -> Result<(tokio::net::tcp::OwnedWriteHalf, BufReader<tokio::net::tcp::OwnedReadHalf>), Box<dyn std::error::Error>> {
    let addr = format!("{}:{}", host, port);
    let stream = TcpStream::connect(&addr).await?;
    let (reader, writer) = stream.into_split();
    let buf_reader = BufReader::new(reader);
    Ok((writer, buf_reader))
}

fn parse_args(args: &[String]) -> (String, String, String, Vec<String>) {
    let mut host = String::from("127.0.0.1");
    let mut port = String::from("7898");
    let mut password = String::new();
    let mut i = 1;

    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--host" => {
                i += 1;
                if i < args.len() {
                    host = args[i].clone();
                }
            }
            "-p" | "--port" => {
                i += 1;
                if i < args.len() {
                    port = args[i].clone();
                }
            }
            "-a" | "--auth" => {
                i += 1;
                if i < args.len() {
                    password = args[i].clone();
                }
            }
            _ => {
                let cmd_args = args[i..].to_vec();
                return (host, port, password, cmd_args);
            }
        }
        i += 1;
    }

    (host, port, password, vec![])
}

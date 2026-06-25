use std::env;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let (host, port, cmd_args) = parse_args(&args);

    let addr = format!("{}:{}", host, port);
    let mut stream = TcpStream::connect(&addr).await?;
    let (reader, mut writer) = stream.split();
    let mut buf_reader = BufReader::new(reader);
    let mut line = String::new();

    if !cmd_args.is_empty() {
        // one-shot mode: send cmd, print response, exit
        for cmd in &cmd_args {
            writer.write_all(cmd.as_bytes()).await?;
            writer.write_all(b"\r\n").await?;
        }
        buf_reader.read_line(&mut line).await?;
        print!("{}", line);
    } else {
        // interactive REPL mode
        let stdin = tokio::io::stdin();
        let mut stdin_reader = BufReader::new(stdin);
        let mut input = String::new();

        println!("Connected to {addr}");
        loop {
            input.clear();
            let n = stdin_reader.read_line(&mut input).await?;
            if n == 0 {
                break;
            }

            let trimmed = input.trim();
            if trimmed.eq_ignore_ascii_case("exit") || trimmed.eq_ignore_ascii_case("quit") {
                break;
            }
            if trimmed.is_empty() {
                continue;
            }

            writer.write_all(trimmed.as_bytes()).await?;
            writer.write_all(b"\r\n").await?;

            line.clear();
            buf_reader.read_line(&mut line).await?;
            print!("{}", line);
        }
    }

    Ok(())
}

fn parse_args(args: &[String]) -> (String, String, Vec<String>) {
    let mut host = String::from("127.0.0.1");
    let mut port = String::from("7898");
    let mut cmd_args: Vec<String> = Vec::new();
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
            _ => {
                cmd_args = args[i..].to_vec();
                break;
            }
        }
        i += 1;
    }

    (host, port, cmd_args)
}

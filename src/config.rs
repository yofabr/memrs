use std::env;

type int = i32;

#[derive(Debug, Clone, Default)]
pub struct Config {
    port: int,
    password: String,
    // size:
}

impl Config {
    pub fn load_config() -> Self {
        let mut config = Self {
            port: 7898,
            password: String::new(),
        };

        if let Ok(port) = env::var("PORT") {
            config.port = port.parse::<i32>().unwrap();
        }
        if let Ok(password) = env::var("PASSWORD") {
            config.password = password;
        }
        config
    }

    pub fn print_config(&self) {
        println!("{:?}", self);
    }
}

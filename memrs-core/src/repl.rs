use color_eyre::{eyre::eyre, Result};

pub enum ReplCommands {
    // GET -> returns the value for the provided key.
    GET(String),

    // SET ->  inserts or updated a key-value pair
    SET(String, String),

    // EXISTS -> Checks if the provided ket exists in a memory.
    EXISTS(String),
    // DEL -> Deletes key value pair
    DEL(String),

    // HSET -> Sets the value of a specific field within a hash.
    HSET(String, String, String),

    // HGET -> Retrieves the value of a specific field.
    HGET(String, String),

    // LPUSH -> Prepends an element to the head (left side) of a list.
    LPUSH(String, String),

    // RPUSH -> Appends an element to the tail (right side) of a list.
    RPUSH(String, String),
    LPOP(String), // Removes and returns the first element from the left.
    RPOP(String), // Removes and returns the last element from the right.
    PING,         // -> PONG
}

impl ReplCommands {
    pub fn parse_command(command: String) -> Result<Self> {
        let trimmed = command.trim();
        if trimmed.is_empty() {
            return Err(eyre!("Empty command detected"));
        }
        let mut words = trimmed.split_whitespace();

        // 2. The first word is always the verb/action.
        // We unwrap safely because we already verified the string isn't empty.
        let verb = words.next().unwrap();

        // 3. Match on the verb and draw arguments out of the iterator as needed
        match verb.to_lowercase().as_str() {
            "get" => {
                let key = words.next().ok_or_else(|| {
                    eyre!("Missing argument — Usage: GET <key>")
                })?;
                Ok(Self::GET(key.to_string()))
            }
            "set" => {
                let key = words.next().ok_or_else(|| {
                    eyre!("Missing argument — Usage: SET <key> <value>")
                })?;
                let value = words.next().ok_or_else(|| {
                    eyre!("Missing argument — Usage: SET <key> <value>")
                })?;
                Ok(Self::SET(key.to_string(), value.to_string()))
            }
            "exists" => {
                let key = words.next().ok_or_else(|| {
                    eyre!("Missing argument — Usage: EXISTS <key>")
                })?;
                Ok(Self::EXISTS(key.to_string()))
            }
            "del" => {
                let key = words.next().ok_or_else(|| {
                    eyre!("Missing argument — Usage: DEL <key>")
                })?;
                Ok(Self::DEL(key.to_string()))
            }
            "hset" => {
                let key = words.next().ok_or_else(|| {
                    eyre!("Missing argument — Usage: HSET <key> <field> <value>")
                })?;
                let field = words.next().ok_or_else(|| {
                    eyre!("Missing argument — Usage: HSET <key> <field> <value>")
                })?;
                let value = words.next().ok_or_else(|| {
                    eyre!("Missing argument — Usage: HSET <key> <field> <value>")
                })?;
                Ok(Self::HSET(key.to_string(), field.to_string(), value.to_string()))
            }
            "hget" => {
                let key = words.next().ok_or_else(|| {
                    eyre!("Missing argument — Usage: HGET <key> <field>")
                })?;
                let field = words.next().ok_or_else(|| {
                    eyre!("Missing argument — Usage: HGET <key> <field>")
                })?;
                Ok(Self::HGET(key.to_string(), field.to_string()))
            }
            "lpush" => {
                let key = words.next().ok_or_else(|| {
                    eyre!("Missing argument — Usage: LPUSH <key> <value>")
                })?;
                let value = words.next().ok_or_else(|| {
                    eyre!("Missing argument — Usage: LPUSH <key> <value>")
                })?;
                Ok(Self::LPUSH(key.to_string(), value.to_string()))
            }
            "rpush" => {
                let key = words.next().ok_or_else(|| {
                    eyre!("Missing argument — Usage: RPUSH <key> <value>")
                })?;
                let value = words.next().ok_or_else(|| {
                    eyre!("Missing argument — Usage: RPUSH <key> <value>")
                })?;
                Ok(Self::RPUSH(key.to_string(), value.to_string()))
            }
            "lpop" => {
                let key = words.next().ok_or_else(|| {
                    eyre!("Missing argument — Usage: LPOP <key>")
                })?;
                Ok(Self::LPOP(key.to_string()))
            }
            "rpop" => {
                let key = words.next().ok_or_else(|| {
                    eyre!("Missing argument — Usage: RPOP <key>")
                })?;
                Ok(Self::RPOP(key.to_string()))
            }
            "ping" => Ok(Self::PING),
            _ => Err(eyre!("Unknown command: {} — Usage: GET | SET | EXISTS | DEL | HSET | HGET | LPUSH | RPUSH | LPOP | RPOP | PING", verb)),
        }
    }
}

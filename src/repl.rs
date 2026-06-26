use color_eyre::{eyre::eyre, Result};

pub enum ReplCommands {
    // GET -> returns the value for the provided key.
    GET(String),

    // SET ->  inserts or updated a key-value pair
    SET,

    // EXISTS -> Checks if the provided ket exists in a memory.
    EXISTS,
    // DEL -> Deletes key value pair
    DEL,

    // HSET -> Sets the value of a specific field within a hash.
    HSET,

    // HGET -> Retrieves the value of a specific field.
    HGET,

    // LPUSH -> Prepends an element to the head (left side) of a list.
    LPUSH,

    // RPUSH -> Appends an element to the tail (right side) of a list.
    RPUSH,
    LPOP, // Removes and returns the first element from the left.
    RPOP, // Removes and returns the last element from the right.
    PING, // -> PONG
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
                let key = words.next().ok_or_else(|| eyre!("GET requires a key"))?;
                Ok(Self::GET(key.to_string()))
            }
            _ => Err(eyre!("Unknown command: {}", verb)),
        }
    }
}

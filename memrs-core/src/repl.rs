use color_eyre::{eyre::eyre, Result};

#[derive(Debug, Clone, PartialEq)]
pub enum ReplCommands {
    // GET -> returns the value for the provided key.
    GET(String),

    // SET ->  inserts or updated a key-value pair
    // optional TTL in seconds
    SET(String, String, Option<u64>),

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
    FLUSHALL,     // Clears all keys
    LISTALL(Option<usize>), // Lists all keys with pagination (10 per page)
    EXPIRE(String, u64), // Sets a timeout on a key in seconds
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
                    eyre!("Missing argument — Usage: SET <key> <value> [ttl_seconds]")
                })?;
                let value = words.next().ok_or_else(|| {
                    eyre!("Missing argument — Usage: SET <key> <value> [ttl_seconds]")
                })?;
                let ttl = words.next().and_then(|s| s.parse::<u64>().ok());
                Ok(Self::SET(key.to_string(), value.to_string(), ttl))
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
            "flushall" => Ok(Self::FLUSHALL),
            "listall" => {
                let page = words.next().map(|p| p.parse::<usize>().unwrap_or(1));
                Ok(Self::LISTALL(page))
            }
            "expire" => {
                let key = words.next().ok_or_else(|| {
                    eyre!("Missing argument — Usage: EXPIRE <key> <seconds>")
                })?;
                let seconds = words.next().ok_or_else(|| {
                    eyre!("Missing argument — Usage: EXPIRE <key> <seconds>")
                })?.parse::<u64>().map_err(|_| eyre!("EXPIRE seconds must be a number — Usage: EXPIRE <key> <seconds>"))?;
                Ok(Self::EXPIRE(key.to_string(), seconds))
            }
            _ => Err(eyre!("Unknown command: {} — Usage: GET | SET | EXISTS | DEL | HSET | HGET | LPUSH | RPUSH | LPOP | RPOP | PING | FLUSHALL | LISTALL | EXPIRE", verb)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_get() {
        let result = ReplCommands::parse_command("GET mykey".into()).unwrap();
        assert_eq!(result, ReplCommands::GET("mykey".into()));
    }

    #[test]
    fn parse_set() {
        let result = ReplCommands::parse_command("SET k v".into()).unwrap();
        assert_eq!(result, ReplCommands::SET("k".into(), "v".into(), None));
    }

    #[test]
    fn parse_set_with_ttl() {
        let result = ReplCommands::parse_command("SET k v 100".into()).unwrap();
        assert_eq!(
            result,
            ReplCommands::SET("k".into(), "v".into(), Some(100))
        );
    }

    #[test]
    fn parse_set_invalid_ttl_is_ignored() {
        let result = ReplCommands::parse_command("SET k v abc".into()).unwrap();
        assert_eq!(
            result,
            ReplCommands::SET("k".into(), "v".into(), None)
        );
    }

    #[test]
    fn parse_exists() {
        let result = ReplCommands::parse_command("EXISTS k".into()).unwrap();
        assert_eq!(result, ReplCommands::EXISTS("k".into()));
    }

    #[test]
    fn parse_del() {
        let result = ReplCommands::parse_command("DEL k".into()).unwrap();
        assert_eq!(result, ReplCommands::DEL("k".into()));
    }

    #[test]
    fn parse_hset() {
        let result = ReplCommands::parse_command("HSET k f v".into()).unwrap();
        assert_eq!(
            result,
            ReplCommands::HSET("k".into(), "f".into(), "v".into())
        );
    }

    #[test]
    fn parse_hget() {
        let result = ReplCommands::parse_command("HGET k f".into()).unwrap();
        assert_eq!(result, ReplCommands::HGET("k".into(), "f".into()));
    }

    #[test]
    fn parse_lpush() {
        let result = ReplCommands::parse_command("LPUSH k v".into()).unwrap();
        assert_eq!(result, ReplCommands::LPUSH("k".into(), "v".into()));
    }

    #[test]
    fn parse_rpush() {
        let result = ReplCommands::parse_command("RPUSH k v".into()).unwrap();
        assert_eq!(result, ReplCommands::RPUSH("k".into(), "v".into()));
    }

    #[test]
    fn parse_lpop() {
        let result = ReplCommands::parse_command("LPOP k".into()).unwrap();
        assert_eq!(result, ReplCommands::LPOP("k".into()));
    }

    #[test]
    fn parse_rpop() {
        let result = ReplCommands::parse_command("RPOP k".into()).unwrap();
        assert_eq!(result, ReplCommands::RPOP("k".into()));
    }

    #[test]
    fn parse_ping() {
        let result = ReplCommands::parse_command("PING".into()).unwrap();
        assert_eq!(result, ReplCommands::PING);
    }

    #[test]
    fn parse_flushall() {
        let result = ReplCommands::parse_command("FLUSHALL".into()).unwrap();
        assert_eq!(result, ReplCommands::FLUSHALL);
    }

    #[test]
    fn parse_listall() {
        let result = ReplCommands::parse_command("LISTALL".into()).unwrap();
        assert_eq!(result, ReplCommands::LISTALL(None));
    }

    #[test]
    fn parse_listall_with_page() {
        let result = ReplCommands::parse_command("LISTALL 3".into()).unwrap();
        assert_eq!(result, ReplCommands::LISTALL(Some(3)));
    }

    #[test]
    fn parse_expire() {
        let result = ReplCommands::parse_command("EXPIRE k 100".into()).unwrap();
        assert_eq!(result, ReplCommands::EXPIRE("k".into(), 100));
    }

    #[test]
    fn parse_case_insensitive() {
        let r1 = ReplCommands::parse_command("GET k".into()).unwrap();
        let r2 = ReplCommands::parse_command("get k".into()).unwrap();
        let r3 = ReplCommands::parse_command("GeT k".into()).unwrap();
        assert_eq!(r1, ReplCommands::GET("k".into()));
        assert_eq!(r2, r1);
        assert_eq!(r3, r1);
    }

    #[test]
    fn parse_extra_whitespace() {
        let result = ReplCommands::parse_command("  SET   k   v  ".into()).unwrap();
        assert_eq!(result, ReplCommands::SET("k".into(), "v".into(), None));
    }

    #[test]
    fn parse_empty_string() {
        assert!(ReplCommands::parse_command("".into()).is_err());
    }

    #[test]
    fn parse_whitespace_only() {
        assert!(ReplCommands::parse_command("   ".into()).is_err());
    }

    #[test]
    fn parse_unknown_command() {
        assert!(ReplCommands::parse_command("FOOBAR".into()).is_err());
    }

    #[test]
    fn parse_get_missing_key() {
        assert!(ReplCommands::parse_command("GET".into()).is_err());
    }

    #[test]
    fn parse_set_missing_value() {
        assert!(ReplCommands::parse_command("SET k".into()).is_err());
    }

    #[test]
    fn parse_hset_missing_args() {
        assert!(ReplCommands::parse_command("HSET k".into()).is_err());
    }

    #[test]
    fn parse_expire_missing_seconds() {
        assert!(ReplCommands::parse_command("EXPIRE k".into()).is_err());
    }

    #[test]
    fn parse_expire_invalid_seconds() {
        assert!(ReplCommands::parse_command("EXPIRE k abc".into()).is_err());
    }

    #[test]
    fn parse_hget_missing_field() {
        assert!(ReplCommands::parse_command("HGET k".into()).is_err());
    }
}

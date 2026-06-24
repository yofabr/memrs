pub enum REPL_COMMANDS {
    // GET -> returns the value for the provided key.
    GET,

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

CREATE TABLE IF NOT EXISTS chats (
    chat_id INTEGER NOT NULL,
    summary TEXT DEFAULT '',
    is_active BOOLEAN NOT NULL DEFAULT 0,
    PRIMARY KEY (chat_id)
);

CREATE TABLE IF NOT EXISTS messages (
    chat_id INTEGER NOT NULL,
    message_id INTEGER NOT NULL,
    user_id INTEGER,
    user_name TEXT,
    content TEXT NOT NULL,
    reply_to_id INTEGER,
    quote TEXT,
    forward_from TEXT,
    timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (chat_id, message_id),
    FOREIGN KEY(chat_id) REFERENCES chats(chat_id)
);

CREATE INDEX IF NOT EXISTS idx_messages_chat ON messages(chat_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_messages_unique_msg ON messages(chat_id, message_id);
CREATE TABLE IF NOT EXISTS threads (
    chat_id INTEGER NOT NULL,
    thread_id INTEGER NOT NULL DEFAULT 0, -- 0 для основного чата или лички
    summary TEXT DEFAULT '',
    PRIMARY KEY (chat_id, thread_id)
);

CREATE TABLE IF NOT EXISTS messages (
    chat_id INTEGER NOT NULL,
    thread_id INTEGER NOT NULL DEFAULT 0,
    message_id INTEGER NOT NULL,
    reply_to_id INTEGER,
    quote TEXT,
    user_id INTEGER,
    user_name TEXT,
    content TEXT NOT NULL,
    timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (chat_id, message_id),
    FOREIGN KEY(chat_id, thread_id) REFERENCES threads(chat_id, thread_id)
);

CREATE INDEX IF NOT EXISTS idx_messages_thread ON messages(chat_id, thread_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_messages_unique_msg ON messages(chat_id, message_id);
CREATE TABLE IF NOT EXISTS threads (
    chat_id INTEGER NOT NULL,
    thread_id INTEGER NOT NULL DEFAULT 0, -- 0 для основного чата или лички
    summary TEXT DEFAULT '',
    last_summarized_id INTEGER DEFAULT 0,
    PRIMARY KEY (chat_id, thread_id)
);

CREATE TABLE IF NOT EXISTS messages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    chat_id INTEGER NOT NULL,
    thread_id INTEGER NOT NULL DEFAULT 0,
    tg_message_id INTEGER NOT NULL,
    reply_to_id INTEGER, -- ID сообщения, на которое ответили
    user_id INTEGER,    -- ID юзера
    user_name TEXT,     -- Ник или имя для контекста
    content TEXT NOT NULL,
    timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY(chat_id, thread_id) REFERENCES threads(chat_id, thread_id)
);

CREATE INDEX IF NOT EXISTS idx_messages_thread ON messages(chat_id, thread_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_messages_unique_msg ON messages(chat_id, tg_message_id);
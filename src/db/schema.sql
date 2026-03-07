CREATE TABLE IF NOT EXISTS chats (
    chat_id INTEGER PRIMARY KEY,
    summary TEXT DEFAULT '',
    last_summarized_id INTEGER DEFAULT 0
);

-- Таблица сообщений
CREATE TABLE IF NOT EXISTS messages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    chat_id INTEGER NOT NULL,
    tg_message_id INTEGER NOT NULL,
    role TEXT NOT NULL, -- 'user' или 'model'
    content TEXT NOT NULL,
    timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY(chat_id) REFERENCES chats(chat_id)
);

-- Индексы для ускорения поиска по чату
CREATE INDEX IF NOT EXISTS idx_messages_chat_id ON messages(chat_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_messages_unique_msg ON messages(chat_id, tg_message_id);
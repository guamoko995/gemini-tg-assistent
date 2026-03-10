use crate::models::{ChatContext, ChatMessage, Role};
use sqlx::{Acquire, Row, Sqlite};

/// Сохраняет сообщение или обновляет его текст, если ID совпадает.
/// Автоматически создает запись о чате, если её нет.
pub async fn upsert_message<'a, A>(
    conn: A,
    chat_id: i64,
    tg_message_id: i64,
    role: Role,
    from: &str,
    content: &str,
) -> Result<(), sqlx::Error>
where
    A: Acquire<'a, Database = Sqlite>,
{
    let mut conn = conn.acquire().await?;

    // Гарантируем наличие чата
    sqlx::query("INSERT INTO chats (chat_id) VALUES (?) ON CONFLICT DO NOTHING")
        .bind(chat_id)
        .execute(&mut *conn)
        .await?;

    // Вставляем или обновляем сообщение
    sqlx::query(
        "INSERT INTO messages (chat_id, tg_message_id, role, user, content) \
         VALUES (?, ?, ?, ?, ?) \
         ON CONFLICT(chat_id, tg_message_id) DO UPDATE SET content = excluded.content",
    )
    .bind(chat_id)
    .bind(tg_message_id)
    .bind(role)
    .bind(from)
    .bind(content)
    .execute(&mut *conn)
    .await?;

    Ok(())
}

/// Собирает текущее саммари и все сообщения, которые еще не были в него включены.
pub async fn get_chat_context<'a, A>(conn: A, chat_id: i64) -> Result<ChatContext, sqlx::Error>
where
    A: Acquire<'a, Database = Sqlite>,
{
    let mut conn = conn.acquire().await?;

    // 1. Получаем состояние "горизонта" памяти
    let chat_row = sqlx::query("SELECT summary, last_summarized_id FROM chats WHERE chat_id = ?")
        .bind(chat_id)
        .fetch_optional(&mut *conn)
        .await?;

    let (summary, last_id) = match chat_row {
        Some(row) => (
            row.get::<Option<String>, _>("summary").unwrap_or_default(),
            row.get::<i64, _>("last_summarized_id"),
        ),
        None => (String::new(), 0i64),
    };

    // 2. Выбираем только новые сообщения (после последнего саммари)
    let messages = sqlx::query_as::<_, ChatMessage>(
        "SELECT role, user, content FROM messages WHERE chat_id = ? AND id > ? ORDER BY id ASC",
    )
    .bind(chat_id)
    .bind(last_id)
    .fetch_all(&mut *conn)
    .await?;

    //println!("{messages:?}");

    Ok(ChatContext { summary, messages })
}

/// Обновляет сжатую память чата и сдвигает указатель последнего обработанного сообщения.
pub async fn update_chat_summary<'a, A>(
    conn: A,
    chat_id: i64,
    new_summary: &str,
    last_id: i64,
) -> Result<(), sqlx::Error>
where
    A: Acquire<'a, Database = Sqlite>,
{
    let mut conn = conn.acquire().await?;

    sqlx::query("UPDATE chats SET summary = ?, last_summarized_id = ? WHERE chat_id = ?")
        .bind(new_summary)
        .bind(last_id)
        .bind(chat_id)
        .execute(&mut *conn)
        .await?;

    Ok(())
}

/// Удаляет сообщения, которые уже были успешно перенесены в саммари.
pub async fn cleanup_summarized_messages<'a, A>(
    conn: A,
    chat_id: i64,
    last_id: i64,
) -> Result<u64, sqlx::Error>
where
    A: Acquire<'a, Database = Sqlite>,
{
    let mut conn = conn.acquire().await?;

    let result = sqlx::query("DELETE FROM messages WHERE chat_id = ? AND id <= ?")
        .bind(chat_id)
        .bind(last_id)
        .execute(&mut *conn)
        .await?;

    Ok(result.rows_affected())
}

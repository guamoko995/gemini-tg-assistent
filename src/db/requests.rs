use crate::models::{ChatContext, ChatMessage};
use sqlx::{Acquire, Row, Sqlite};

/// Сохраняет сообщение или обновляет его текст, если ID совпадает.
/// Автоматически создает запись о чате, если её нет.
pub async fn upsert_message<'a, A>(
    conn: A,
    chat_id: i64,
    thread_id: i64,
    msg: &ChatMessage,
) -> Result<i64, sqlx::Error>
where
    A: Acquire<'a, Database = Sqlite>,
{
    let mut conn = conn.acquire().await?;

    sqlx::query("INSERT INTO threads (chat_id, thread_id) VALUES (?, ?) ON CONFLICT DO NOTHING")
        .bind(chat_id)
        .bind(thread_id)
        .execute(&mut *conn)
        .await?;

    sqlx::query(
        r#"
        INSERT INTO messages (
            chat_id, thread_id, message_id, reply_to_id, user_id, user_name, content, quote, timestamp,
            forward_from
        ) 
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(chat_id, message_id) DO UPDATE SET 
            content = excluded.content
        "#,
    )
    .bind(chat_id)
    .bind(thread_id)
    .bind(msg.message_id)
    .bind(msg.reply_to_id)
    .bind(msg.user_id)
    .bind(&msg.user_name)
    .bind(&msg.content)
    .bind(&msg.quote)
    .bind(msg.timestamp)
    .bind(&msg.forward_from)
    .execute(&mut *conn)
    .await?;

    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM messages WHERE chat_id = ? AND thread_id = ?")
            .bind(chat_id)
            .bind(thread_id)
            .fetch_one(&mut *conn)
            .await?;

    Ok(count)
}

/// Собирает текущее саммари и все сообщения, которые еще не были в него включены.
pub async fn get_chat_context<'a, A>(
    conn: A,
    chat_id: i64,
    thread_id: i64, // Добавили тред
) -> Result<ChatContext, sqlx::Error>
where
    A: Acquire<'a, Database = Sqlite>,
{
    let mut conn = conn.acquire().await?;

    // 1. Получаем саммари и точку отсечки для конкретного треда
    let thread_row = sqlx::query("SELECT summary FROM threads WHERE chat_id = ? AND thread_id = ?")
        .bind(chat_id)
        .bind(thread_id)
        .fetch_optional(&mut *conn)
        .await?;

    let summary = match thread_row {
        Some(row) => {
            let a = row.get::<Option<String>, _>("summary").unwrap_or_default();
            a
        }
        None => String::new(),
    };

    // 2. Выбираем сообщения только из этого треда, которые еще не попали в саммари
    let messages = sqlx::query_as::<_, ChatMessage>(
        r#"
        SELECT message_id, reply_to_id, user_id, user_name, content, quote, timestamp, forward_from 
        FROM messages 
        WHERE chat_id = ? AND thread_id = ? 
        ORDER BY message_id ASC
        LIMIT 200
        "#,
    )
    .bind(chat_id)
    .bind(thread_id)
    .fetch_all(&mut *conn)
    .await?;

    Ok(ChatContext { summary, messages })
}

pub async fn archive_thread_messages<'a, A>(
    conn: A,
    chat_id: i64,
    thread_id: i64,
    new_summary: &str,
    last_id: i64,
) -> Result<u64, sqlx::Error>
where
    A: Acquire<'a, Database = Sqlite>,
{
    let mut conn = conn.acquire().await?;
    let mut tx = conn.begin().await?;

    sqlx::query("UPDATE threads SET summary = ? WHERE chat_id = ? AND thread_id = ?")
        .bind(new_summary)
        .bind(chat_id)
        .bind(thread_id)
        .execute(&mut *tx)
        .await?;

    let result =
        sqlx::query("DELETE FROM messages WHERE chat_id = ? AND thread_id = ? AND message_id <= ?")
            .bind(chat_id)
            .bind(thread_id)
            .bind(last_id)
            .execute(&mut *tx)
            .await?;

    let rows_deleted = result.rows_affected();
    tx.commit().await?;

    Ok(rows_deleted)
}

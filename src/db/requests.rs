use crate::models::{ChatContext, ChatMessage};
use sqlx::{Acquire, Row, Sqlite};

/// Сохраняет сообщение или обновляет его текст, если ID совпадает.
/// Автоматически создает запись о чате, если её нет.
pub async fn upsert_message<'a, A>(
    conn: A,
    chat_id: i64,
    thread_id: i64,
    msg: &ChatMessage, // Передаем ссылку на нашу новую сущность
) -> Result<(), sqlx::Error>
where
    A: Acquire<'a, Database = Sqlite>,
{
    let mut conn = conn.acquire().await?;

    // Гарантируем наличие записи в таблице тредов
    sqlx::query("INSERT INTO threads (chat_id, thread_id) VALUES (?, ?) ON CONFLICT DO NOTHING")
        .bind(chat_id)
        .bind(thread_id)
        .execute(&mut *conn)
        .await?;

    // Вставляем или обновляем сообщение, используя поля из ChatMessage
    sqlx::query(
        r#"
        INSERT INTO messages (
            chat_id, thread_id, tg_message_id, reply_to_id, user_id, user_name, content, timestamp
        ) 
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(chat_id, tg_message_id) DO UPDATE SET 
            content = excluded.content,
            user_name = excluded.user_name
        "#,
    )
    .bind(chat_id)
    .bind(thread_id)
    .bind(msg.tg_message_id)
    .bind(msg.reply_to_id)
    .bind(msg.user_id)
    .bind(&msg.user_name)
    .bind(&msg.content)
    .bind(msg.timestamp) // sqlx сам поймет, как это положить в DATETIME
    .execute(&mut *conn)
    .await?;

    Ok(())
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
    let thread_row = sqlx::query(
        "SELECT summary, last_summarized_id FROM threads WHERE chat_id = ? AND thread_id = ?",
    )
    .bind(chat_id)
    .bind(thread_id)
    .fetch_optional(&mut *conn)
    .await?;

    let (summary, last_id) = match thread_row {
        Some(row) => (
            row.get::<Option<String>, _>("summary").unwrap_or_default(),
            row.get::<i64, _>("last_summarized_id"),
        ),
        None => (String::new(), 0i64),
    };

    // 2. Выбираем сообщения только из этого треда, которые еще не попали в саммари
    let messages = sqlx::query_as::<_, ChatMessage>(
        r#"
        SELECT tg_message_id, reply_to_id, user_id, user_name, content, timestamp 
        FROM messages 
        WHERE chat_id = ? AND thread_id = ? AND id > ? 
        ORDER BY id ASC
        "#,
    )
    .bind(chat_id)
    .bind(thread_id)
    .bind(last_id)
    .fetch_all(&mut *conn)
    .await?;

    Ok(ChatContext {
        chat_id,
        thread_id,
        summary,
        messages,
    })
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
    // Начинаем транзакцию, чтобы всё прошло атомарно
    let mut tx = conn.begin().await?;

    // 1. Обновляем саммари в таблице threads
    sqlx::query(
        "UPDATE threads SET summary = ?, last_summarized_id = ? \
         WHERE chat_id = ? AND thread_id = ?",
    )
    .bind(new_summary)
    .bind(last_id)
    .bind(chat_id)
    .bind(thread_id)
    .execute(&mut *tx)
    .await?;

    // 2. Удаляем сообщения, которые теперь "внутри" саммари
    let result =
        sqlx::query("DELETE FROM messages WHERE chat_id = ? AND thread_id = ? AND id <= ?")
            .bind(chat_id)
            .bind(thread_id)
            .bind(last_id)
            .execute(&mut *tx)
            .await?;

    let rows_deleted = result.rows_affected();

    // Фиксируем изменения
    tx.commit().await?;

    Ok(rows_deleted)
}

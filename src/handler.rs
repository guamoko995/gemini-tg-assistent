use crate::db;
use crate::gemini::GeminiClient;
use crate::models::Role;

use sqlx::SqlitePool;
use std::sync::Arc;
use teloxide::prelude::*;
use teloxide::types::ReplyParameters;

/// Основной обработчик входящих сообщений
pub async fn message_handler(
    bot: Bot,
    msg: Message,
    pool: Arc<SqlitePool>,
    gemini: Arc<GeminiClient>,
) -> ResponseResult<()> {
    let chat_id = msg.chat.id.0;
    let bot_user = bot.get_me().await?;
    let bot_username = bot_user.user.username.as_deref().unwrap_or("bot");

    // 1. Извлекаем текст
    let text = match msg.text() {
        Some(t) => t,
        None => return Ok(()), // Игнорируем стикеры/фото для минимализма
    };

    // 2. Сохраняем входящее сообщение в базу (всегда, для контекста)
    let tg_msg_id = msg.id.0 as i64;
    if let Err(e) = db::upsert_message(&*pool, chat_id, tg_msg_id, Role::User, text).await {
        log::error!("Ошибка сохранения сообщения: {:?}", e);
    }

    // 3. Проверяем, нужно ли отвечать (тег бота или ответ на его сообщение)
    let is_mentioned = text.contains(&format!("@{}", bot_username));
    let is_reply_to_bot = msg
        .reply_to_message()
        .and_then(|m| m.from.clone())
        .map(|u| u.id == bot_user.user.id)
        .unwrap_or(false);

    if is_mentioned || is_reply_to_bot {
        // Показываем статус "печатает"
        bot.send_chat_action(msg.chat.id, teloxide::types::ChatAction::Typing)
            .await?;

        // 4. Получаем контекст из БД
        let context = match db::get_chat_context(&*pool, chat_id).await {
            Ok(c) => c,
            Err(e) => {
                log::error!("Ошибка получения контекста: {:?}", e);
                return Ok(());
            }
        };

        // 5. Запрос к Gemini
        match gemini.ask(&context.summary, &context.messages).await {
            Ok(ai_response) => {
                // 6. Отправляем ответ пользователю
                let sent_msg = bot
                    .send_message(msg.chat.id, &ai_response)
                    .reply_parameters(ReplyParameters::new(msg.id))
                    .await?;

                // 7. Сохраняем ответ бота в базу
                let bot_msg_id = sent_msg.id.0 as i64;
                let _ = db::upsert_message(&*pool, chat_id, bot_msg_id, Role::Model, &ai_response)
                    .await;
            }
            Err(e) => {
                log::error!("Ошибка Gemini: {:?}", e);
                bot.send_message(msg.chat.id, "Извини, я временно не могу сообразить...")
                    .await?;
            }
        }
    }

    Ok(())
}

use axiomtrade_rs::auth::{AuthClient, TokenManager, AuthTokens};
use chrono::Utc;
use std::path::PathBuf;

#[tokio::main]
async fn main() {
    let tokens_file = PathBuf::from(".axiom_tokens.json");

    // Кладём refresh_token туда, откуда AuthClient его подхватит
    let seed = TokenManager::new(Some(tokens_file.clone()));
    seed.set_tokens(AuthTokens {
        access_token: String::new(),
        refresh_token: "ВСТАВЬ_СЮДА_auth-refresh-token".to_string(),
        expires_at: Some(Utc::now()),
    }).await.unwrap();

    let mut auth_client = AuthClient::new().unwrap();
    let tokens = match auth_client.refresh_tokens().await {
        Ok(t) => t,
        Err(e) => {
            println!("Не удалось обновить токен: {}", e);
            return;
        }
    };

    println!("✓ Токен выпущен: {}...", &tokens.access_token[..20.min(tokens.access_token.len())]);

    // если файл реально не нужен — можешь удалить его сразу после запуска
    // let _ = std::fs::remove_file(&tokens_file);
}
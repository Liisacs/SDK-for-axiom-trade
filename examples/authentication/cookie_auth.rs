/// Cookie Authentication Example (real, working version)
///
/// Изначальный пример в репозитории был муляжом: он логинился по
/// email/password, а потом просто печатал захардкоженные фейковые строки
/// вроде "access_cookie_value" — реальных кук там не было вообще.
///
/// В реальности же (см. src/auth/client.rs) то, что SDK называет
/// `access_token` / `refresh_token`, — это буквально значения браузерных
/// кук `auth-access-token` и `auth-refresh-token` c axiom.trade:
/// make_authenticated_request() отправляет их именно как заголовок
/// `Cookie: auth-access-token=...`. А AuthClient при создании (AuthClient::new())
/// всегда подхватывает файл `.axiom_tokens.json` из текущей папки — если
/// он там уже лежит с валидными значениями, никакого email/пароля/OTP
/// вообще не нужно.
///
/// Значит реальный "логин через куки" — это:
///   1. Залогиниться в axiom.trade в обычном браузере (как обычно).
///   2. DevTools → Application (Chrome) / Storage (Firefox) → Cookies →
///      https://axiom.trade → скопировать значения кук
///      `auth-access-token` и `auth-refresh-token`.
///   3. Положить их в .env (см. ниже) и запустить этот пример — он сам
///      сохранит их в .axiom_tokens.json, откуда их потом заберёт
///      ЛЮБОЙ клиент SDK (PortfolioClient, MarketDataClient и т.д.).
///
/// .env:
///   AXIOM_COOKIE_ACCESS_TOKEN=<значение cookie auth-access-token>
///   AXIOM_COOKIE_REFRESH_TOKEN=<значение cookie auth-refresh-token>
///   # опционально — свой Solana-адрес, чтобы дополнительно проверить баланс:
///   AXIOM_TEST_WALLET=<ваш адрес>
///
/// Запуск: cargo run --example cookie_auth

use axiomtrade_rs::auth::{AuthTokens, TokenManager};
use axiomtrade_rs::api::market_data::MarketDataClient;
use axiomtrade_rs::api::portfolio::PortfolioClient;
use axiomtrade_rs::models::market::TimePeriod;
use std::env;
use std::path::PathBuf;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    println!("Cookie Authentication Example (реальная версия)");
    println!("=================================================\n");

    let access_token = env::var("AXIOM_COOKIE_ACCESS_TOKEN")
        .expect("AXIOM_COOKIE_ACCESS_TOKEN must be set (значение cookie auth-access-token из браузера)");
    let refresh_token = env::var("AXIOM_COOKIE_REFRESH_TOKEN")
        .expect("AXIOM_COOKIE_REFRESH_TOKEN must be set (значение cookie auth-refresh-token из браузера)");

    // Тот же путь, что AuthClient::new_with_user_agent() жёстко использует
    // внутри (см. src/auth/client.rs) — поэтому все остальные клиенты,
    // созданные после этого шага, подхватят сессию автоматически.
    let tokens_path = PathBuf::from(".axiom_tokens.json");
    let token_manager = TokenManager::new(Some(tokens_path.clone()));

    let tokens = AuthTokens {
        access_token,
        refresh_token,
        // Точный срок жизни браузерной сессии неизвестен, поэтому не
        // выставляем — is_expired()/needs_refresh() в этом случае
        // считают токен не истёкшим и просто используют его как есть.
        expires_at: None,
    };

    println!("Шаг 1: Сохраняем куки как сессию SDK...");
    match token_manager.set_tokens(tokens).await {
        Ok(_) => println!("✓ Сохранено в {}\n", tokens_path.display()),
        Err(e) => {
            eprintln!("✗ Не удалось сохранить токены: {e}");
            return;
        }
    }

    // --- Проверка №1: живые публичные рыночные данные ---
    // Этот эндпоинт всё равно требует авторизованный запрос
    // (make_authenticated_request), так что это честная проверка того,
    // что куки реально рабочие, а не просто "файл записался".
    println!("Шаг 2: Проверяем куки реальным запросом (trending tokens, 24ч)...");
    match MarketDataClient::new() {
        Ok(mut market_client) => {
            match market_client.get_trending_tokens(TimePeriod::TwentyFourHours).await {
                Ok(trending) => {
                    println!("✓ Куки рабочие! Получено {} токенов. Топ-5:", trending.len());
                    for (i, t) in trending.iter().take(5).enumerate() {
                        println!(
                            "  {}. {} ({}) — ${:.6}, объём 24ч {:.2} SOL",
                            i + 1,
                            t.symbol,
                            t.name,
                            t.price_usd,
                            t.volume_24h
                        );
                    }
                }
                Err(e) => {
                    eprintln!("✗ Запрос не прошёл: {e}");
                    eprintln!("  Возможные причины: куки устарели (перелогиньтесь в браузере");
                    eprintln!("  и скопируйте свежие значения), либо перепутаны access/refresh.");
                }
            }
        }
        Err(e) => eprintln!("✗ Не удалось создать MarketDataClient: {e}"),
    }

    // --- Проверка №2 (опционально): баланс конкретного кошелька ---
    if let Ok(wallet) = env::var("AXIOM_TEST_WALLET") {
        println!("\nШаг 3: Проверяем баланс кошелька {wallet}...");
        match PortfolioClient::new() {
            Ok(mut portfolio_client) => match portfolio_client.get_balance(&wallet).await {
                Ok(balance) => {
                    println!(
                        "✓ SOL: {:.6}, общая стоимость: ${:.2}, токенов на балансе: {}",
                        balance.sol_balance,
                        balance.total_value_usd,
                        balance.token_balances.len()
                    );
                }
                Err(e) => eprintln!("✗ Не удалось получить баланс: {e}"),
            },
            Err(e) => eprintln!("✗ Не удалось создать PortfolioClient: {e}"),
        }
    } else {
        println!("\n(AXIOM_TEST_WALLET не задан — пропускаем проверку баланса)");
    }

    println!("\nГотово. Сессия сохранена в .axiom_tokens.json — при следующих запусках");
    println!("любых других примеров (get_portfolio, trending_tokens и т.д.) заново");
    println!("указывать куки уже не нужно, пока сессия не истечёт на сервере.");
}

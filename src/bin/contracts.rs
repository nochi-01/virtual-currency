use dotenv::dotenv;
use std::env;
use sqlx::PgPool;
use serde::Deserialize;
use tokio::time::{sleep, Duration};

#[derive(Debug, Deserialize)]
struct Coin {
    id: String,
    platforms: std::collections::HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct CoinDetail {
    id: String,
    symbol: Option<String>,
    name: Option<String>,
    platforms: std::collections::HashMap<String, String>,
    contract_address: Option<String>,
    decimals: Option<i32>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let database_url = env::var("DATABASE_URL")?;
    let pool = PgPool::connect(&database_url).await?;

    let coin_list_url = "https://api.coingecko.com/api/v3/coins/list?include_platform=true";
    let coin_list: Vec<Coin> = reqwest::get(coin_list_url).await?.json().await?;

    for coin in coin_list.iter().take(100) {
        let url = format!("https://api.coingecko.com/api/v3/coins/{}", coin.id);
        let res = reqwest::get(&url).await?;

        match res.json::<CoinDetail>().await {
            Ok(detail) => {
                for (platform, address) in detail.platforms.iter() {
                    if address.is_empty() {
                        continue;
                    }

                    println!("📥 Inserting contract: {} on {}", address, platform);

                    sqlx::query!(
                        r#"
                        INSERT INTO contract.token_info (
                            platform,
                            contract_address,
                            name,
                            symbol,
                            decimals,
                            fetched_at
                        )
                        VALUES ($1, $2, $3, $4, $5, now())
                        "#,
                        Some(platform),
                        address,
                        detail.name,
                        detail.symbol,
                        detail.decimals
                    )
                    .execute(&pool)
                    .await?;
                }
            }
            Err(e) => {
                // レスポンスボディの取得用にもう一度リクエスト
                println!("⚠️ Failed to parse detail for {}: {}", coin.id, e);
            }
        }

        // レート制限回避
        sleep(Duration::from_millis(1500)).await;
    }

    println!("✅ Successfully inserted contracts into contract.token_info!");
    Ok(())
}

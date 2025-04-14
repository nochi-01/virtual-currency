use dotenv::dotenv;
use std::env;
use sqlx::{PgPool, types::BigDecimal};
use num_traits::FromPrimitive;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Exchange {
    id: String,
    name: Option<String>,
    year_established: Option<i32>,
    country: Option<String>,
    trade_volume_24h_btc: Option<f64>,
    trust_score: Option<i32>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // .envから環境変数を読み込み
    dotenv().ok();
    let database_url = env::var("DATABASE_URL")?;
    let pool = PgPool::connect(&database_url).await?;

    // CoinGecko APIから取引所情報を取得
    let url = "https://api.coingecko.com/api/v3/exchanges";
    let exchanges: Vec<Exchange> = reqwest::get(url).await?.json().await?;

    // 各取引所をDBに挿入
    for exchange in exchanges {
        println!("📥 Inserting Exchange: {:?}", exchange);

        let volume_bd = exchange.trade_volume_24h_btc
            .and_then(BigDecimal::from_f64);

        sqlx::query!(
            r#"
            INSERT INTO exchanges.exchange_info (
                id,
                name,
                year_established,
                country,
                trade_volume_24h_btc,
                trust_score,
                fetched_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, now())
            "#,
            exchange.id,
            exchange.name,
            exchange.year_established,
            exchange.country,
            volume_bd,
            exchange.trust_score
        )
        .execute(&pool)
        .await?;
    }

    println!("✅ Successfully inserted exchanges into exchanges.exchange_info!");
    Ok(())
}

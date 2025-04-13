use dotenv::dotenv;
use std::env;
use sqlx::{PgPool, types::BigDecimal};
use num_traits::FromPrimitive;
use serde_json::Value;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // .env読み込み & DB接続
    dotenv().ok();
    let database_url = env::var("DATABASE_URL")?;
    let pool = PgPool::connect(&database_url).await?;

    // DEX Screener API
    let url = "https://api.dexscreener.com/latest/dex/pairs/ethereum/0x8ad599c3a0ff1de082011efddc58f1908eb6e6d8";

    // API呼び出し & JSON取得
    let resp = reqwest::get(url).await?.text().await?;
    let json: Value = serde_json::from_str(&resp)?;

    // データ抽出
    let pair = &json["pair"];
    let exchange = pair["dexId"].as_str().unwrap_or("unknown");
    let token_address = pair["baseToken"]["address"].as_str().unwrap_or("unknown");
    let price = pair["priceUsd"].as_str().unwrap_or("0.0").parse::<f64>().unwrap_or(0.0);
    let liquidity = pair["liquidity"]["usd"].as_str().unwrap_or("0.0").parse::<f64>().unwrap_or(0.0);

    let price_bd = BigDecimal::from_f64(price);
    let liquidity_bd = BigDecimal::from_f64(liquidity);

    // データ挿入
    sqlx::query!(
        r#"
        INSERT INTO onchain.dex_token_prices (
            exchange,
            token_address,
            price,
            liquidity_usd,
            fetched_at
        )
        VALUES ($1, $2, $3, $4, now())
        "#,
        exchange,
        token_address,
        price_bd,
        liquidity_bd
    )
    .execute(&pool)
    .await?;

    println!("✅ Successfully inserted DEX Screener data into onchain.dex_token_prices!");
    Ok(())
}

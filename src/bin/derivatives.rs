use dotenv::dotenv;
use std::env;
use sqlx::{PgPool, types::BigDecimal};
use num_traits::FromPrimitive;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct DerivativeMarket {
    id: Option<String>,
    symbol: Option<String>,
    index_id: Option<String>,
    price: Option<String>,
    contract_type: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // .envファイル読み込み → DB接続
    dotenv().ok();
    let database_url = env::var("DATABASE_URL")?;
    let pool = PgPool::connect(&database_url).await?;

    // CoinGecko API（デリバティブ市場）
    let url = "https://api.coingecko.com/api/v3/derivatives";
    let response = reqwest::get(url).await?.json::<Vec<DerivativeMarket>>().await?;

    for market in response {
        println!("{:?}", market); // ← デバッグ出力で中身を確認

        match &market.id {
            Some(id) => {
                // price: Option<String> → Option<f64> → Option<BigDecimal>
                let price_bd = market
                    .price
                    .as_ref()
                    .and_then(|p| p.parse::<f64>().ok())
                    .and_then(BigDecimal::from_f64);

                sqlx::query!(
                    r#"
                    INSERT INTO derivatives.derivative_markets (
                        id,
                        symbol,
                        index,
                        price,
                        contract_type,
                        fetched_at
                    )
                    VALUES ($1, $2, $3, $4, $5, now())
                    "#,
                    id,
                    market.symbol,
                    market.index_id,
                    price_bd,
                    market.contract_type
                )
                .execute(&pool)
                .await?;
            }
            None => {
                // id がない場合はスキップ（ログは残す）
                println!("⚠️ Skipped a market because id was missing.");
            }
        }
    }

    println!("✅ Successfully inserted into derivatives.derivative_markets!");
    Ok(())
}

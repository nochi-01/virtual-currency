use sqlx::{PgPool, types::BigDecimal as SqlxBigDecimal};
use dotenv::dotenv;
use std::collections::HashMap;
use std::env;
use num_traits::FromPrimitive;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let database_url = env::var("DATABASE_URL")?;
    let pool = PgPool::connect(&database_url).await?;

    // 複数コイン（bitcoin, ethereum, ripple）＋ USD & JPY
    let coin_ids = ["bitcoin", "ethereum", "ripple"];
    let currencies = ["usd", "jpy"];
    let coins_query = coin_ids.join(","); // "bitcoin,ethereum,ripple"
    let currencies_query = currencies.join(","); // "usd,jpy"

    // CoinGecko API URL
    let url = format!(
        "https://api.coingecko.com/api/v3/simple/price?ids={}&vs_currencies={}&include_market_cap=true&include_24hr_vol=true&include_24hr_change=true",
        coins_query, currencies_query
    );

    // CoinGecko APIからJSONを取得
    let resp: HashMap<String, HashMap<String, f64>> = reqwest::get(&url).await?.json().await?;

    // コインごとにループ処理
    for &coin in &coin_ids {
        if let Some(data) = resp.get(coin) {
            for &currency in &currencies {
                let price = data.get(currency).and_then(|v| SqlxBigDecimal::from_f64(*v));
                let market_cap = data.get(&format!("{}_market_cap", currency)).and_then(|v| SqlxBigDecimal::from_f64(*v));
                let volume_24h = data.get(&format!("{}_24h_vol", currency)).and_then(|v| SqlxBigDecimal::from_f64(*v));
                let change_24h = data.get(&format!("{}_24h_change", currency)).and_then(|v| SqlxBigDecimal::from_f64(*v));

                sqlx::query!(
                    r#"
                    INSERT INTO simple.current_price 
                    (id, vs_currency, price, market_cap, volume_24h, change_24h, fetched_at)
                    VALUES ($1, $2, $3, $4, $5, $6, now())
                    "#,
                    coin,
                    currency,
                    price,
                    market_cap,
                    volume_24h,
                    change_24h
                )
                .execute(&pool)
                .await?;
            }
        } else {
            println!("⚠️ No data found for {}", coin);
        }
    }

    println!("✅ Inserted bitcoin, ethereum, ripple for USD & JPY.");
    Ok(())
}

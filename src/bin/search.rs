use dotenv::dotenv;
use std::env;
use sqlx::PgPool;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct TrendingResponse {
    coins: Vec<TrendingCoinWrapper>,
}

#[derive(Debug, Deserialize)]
struct TrendingCoinWrapper {
    item: TrendingCoin,
}

#[derive(Debug, Deserialize)]
struct TrendingCoin {
    id: String,
    name: Option<String>,
    symbol: Option<String>,
    market_cap_rank: Option<i32>,
    score: Option<i32>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // .envファイルからDB接続情報を取得
    dotenv().ok();
    let database_url = env::var("DATABASE_URL")?;
    let pool = PgPool::connect(&database_url).await?;

    // CoinGecko APIからトレンドコインを取得
    let url = "https://api.coingecko.com/api/v3/search/trending";
    let response: TrendingResponse = reqwest::get(url).await?.json().await?;

    // 各コイン情報をDBに挿入
    for coin in response.coins {
        let c = coin.item;

        println!("📥 Inserting Trending Coin: {:?}", c);

        sqlx::query!(
            r#"
            INSERT INTO search.trending_coins (
                id,
                name,
                symbol,
                market_cap_rank,
                score,
                fetched_at
            )
            VALUES ($1, $2, $3, $4, $5, now())
            "#,
            c.id,
            c.name,
            c.symbol,
            c.market_cap_rank,
            c.score,
        )
        .execute(&pool)
        .await?;
    }

    println!("✅ Successfully inserted trending coins into search.trending_coins!");
    Ok(())
}

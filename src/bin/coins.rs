use dotenv::dotenv;
use std::env;
use sqlx::PgPool;
use serde::Deserialize;
use chrono::NaiveDate;

#[derive(Debug, Deserialize)]
struct CoinListItem {
    id: String,
}

#[derive(Debug, Deserialize)]
struct CoinDetail {
    id: String,
    symbol: Option<String>,
    name: Option<String>,
    hashing_algorithm: Option<String>,
    description: Option<Description>,
    links: Option<Links>,
    genesis_date: Option<String>,
    market_cap_rank: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct Description {
    en: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Links {
    homepage: Option<Vec<String>>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // .envから環境変数を読み込み
    dotenv().ok();
    let database_url = env::var("DATABASE_URL")?;
    let pool = PgPool::connect(&database_url).await?;

    // コイン一覧を取得（※レスポンスが正常かチェック）
    let coin_list_url = "https://api.coingecko.com/api/v3/coins/list";
    let res = reqwest::get(coin_list_url).await?;

    if !res.status().is_success() {
        let text = res.text().await?;
        println!("❌ Failed to fetch coin list. Response: {}", text);
        return Ok(());
    }

    let coin_list: Vec<CoinListItem> = res.json().await?;

    // 各コインの詳細を取得してDBに挿入
    for coin in coin_list.iter().take(100) {
        println!("📥 Inserting Coin: {:?}", coin.id);

        let url = format!("https://api.coingecko.com/api/v3/coins/{}", coin.id);
        let res = reqwest::get(&url).await?;

        if !res.status().is_success() {
            let text = res.text().await?;
            println!("⚠️ Failed to fetch coin detail: {}, response: {}", coin.id, text);
            continue;
        }

        let Ok(detail) = res.json::<CoinDetail>().await else {
            println!("⚠️ Failed to parse coin detail: {}", coin.id);
            continue;
        };

        let homepage = detail.links
            .and_then(|l| l.homepage)
            .and_then(|vec| {
                let filtered: Vec<String> = vec.into_iter().filter(|s| !s.is_empty()).collect();
                if filtered.is_empty() {
                    None
                } else {
                    Some(filtered)
                }
            });

        let description = detail.description.and_then(|d| d.en);
        let genesis_date = match detail.genesis_date {
            Some(date_str) => NaiveDate::parse_from_str(&date_str, "%Y-%m-%d").ok(),
            None => None,
        };

        sqlx::query!(
            r#"
            INSERT INTO coins.detail (
                id,
                symbol,
                name,
                hashing_algorithm,
                description,
                homepage,
                genesis_date,
                market_cap_rank,
                fetched_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, now())
            "#,
            detail.id,
            detail.symbol,
            detail.name,
            detail.hashing_algorithm,
            description,
            homepage.as_deref(),
            genesis_date,
            detail.market_cap_rank
        )
        .execute(&pool)
        .await?;
    }

    println!("✅ Successfully inserted coins into coins.detail!");
    Ok(())
}

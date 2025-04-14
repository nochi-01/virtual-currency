use dotenv::dotenv;
use std::env;
use sqlx::PgPool;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Platform {
    id: String,
    name: Option<String>,
    chain_identifier: Option<i32>,
    shortname: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 環境変数（DATABASE_URL）の読み込み
    dotenv().ok();
    let database_url = env::var("DATABASE_URL")?;
    let pool = PgPool::connect(&database_url).await?;

    // CoinGecko APIからアセットプラットフォーム一覧を取得
    let url = "https://api.coingecko.com/api/v3/asset_platforms";
    let platforms: Vec<Platform> = reqwest::get(url).await?.json().await?;

    // 各プラットフォーム情報をDBに挿入
    for platform in platforms {
        println!("📥 Inserting Platform: {:?}", platform.id);

        sqlx::query!(
            r#"
            INSERT INTO asset_platforms.platforms (
                id,
                name,
                chain_identifier,
                shortname,
                fetched_at
            )
            VALUES ($1, $2, $3, $4, now())
            "#,
            platform.id,
            platform.name,
            platform.chain_identifier,
            platform.shortname
        )
        .execute(&pool)
        .await?;
    }

    println!("✅ Successfully inserted platforms into asset_platforms.platforms!");
    Ok(())
}

// .envファイルから環境変数を読み込むためのクレート
use dotenv::dotenv;
// 環境変数（DB接続情報）を取得するための標準ライブラリ
use std::env;
// PostgreSQL用接続プールとBigDecimal型を含むsqlxクレート
use sqlx::{PgPool, types::BigDecimal};
// f64 → BigDecimal への変換に使うクレート
use num_traits::FromPrimitive;
// JSONをRust構造体に変換するためのデリバイブ用クレート
use serde::Deserialize;

// APIレスポンスのカテゴリーデータを受け取るための構造体
#[derive(Debug, Deserialize)]
struct Category {
    id: Option<String>,           // カテゴリーID（API上のID）
    name: Option<String>,         // カテゴリー名
    market_cap: Option<f64>,      // 時価総額（USD）
    volume_24h: Option<f64>,      // 24時間の取引量（USD）
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // .envファイルを読み込み
    dotenv().ok();

    // DATABASE_URL環境変数から接続文字列を取得
    let database_url = env::var("DATABASE_URL")?;

    // PostgreSQLに非同期接続
    let pool = PgPool::connect(&database_url).await?;

    // CoinGeckoのカテゴリーデータAPIエンドポイント
    let url = "https://api.coingecko.com/api/v3/coins/categories";

    // APIからデータ取得してVec<Category>型にデシリアライズ
    let response = reqwest::get(url).await?.json::<Vec<Category>>().await?;

    // 各カテゴリデータを1件ずつ処理
    for category in response {
        // レスポンスを標準出力に表示（デバッグ用）
        println!("{:?}", category);

        // category_idがない場合はスキップ
        let Some(category_id) = &category.id else {
            println!("⚠️ Skipped a category because id was missing.");
            continue;
        };

        // market_cap, volume_24h を Option<f64> → Option<BigDecimal> に変換
        let market_cap_bd = category.market_cap.and_then(BigDecimal::from_f64);
        let volume_24h_bd = category.volume_24h.and_then(BigDecimal::from_f64);

        // PostgreSQLのテーブルにデータを挿入
        sqlx::query!(
            r#"
            INSERT INTO categories.category_market_data (
                category_id,
                name,
                market_cap,
                volume_24h,
                updated_at
            )
            VALUES ($1, $2, $3, $4, now())
            "#,
            category_id,
            category.name,
            market_cap_bd,
            volume_24h_bd
        )
        .execute(&pool)
        .await?;
    }

    // 挿入処理の完了ログ
    println!("✅ Successfully inserted into categories.category_market_data!");
    Ok(())
}

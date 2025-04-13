use dotenv::dotenv;
use std::collections::HashMap;
use std::env;
use sqlx::{PgPool, types::BigDecimal};
use num_traits::FromPrimitive;
use serde::Deserialize;

// CoinGecko APIのglobalエンドポイントのデータ構造に対応
#[derive(Debug, Deserialize)]
struct GlobalData {
    active_cryptocurrencies: Option<i32>,
    upcoming_icos: Option<i32>,
    ongoing_icos: Option<i32>,
    ended_icos: Option<i32>,
    markets: Option<i32>,
    total_market_cap: HashMap<String, f64>,
    total_volume: HashMap<String, f64>,
    market_cap_percentage: HashMap<String, f64>,
}

// APIレスポンス全体構造
#[derive(Debug, Deserialize)]
struct ApiResponse {
    data: GlobalData,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // .envファイルから環境変数を読み込む
    dotenv().ok();

    // DATABASE_URLを取得して接続プールを作成
    let database_url = env::var("DATABASE_URL")?;
    let pool = PgPool::connect(&database_url).await?;

    // グローバルマーケット統計APIを呼び出し
    let url = "https://api.coingecko.com/api/v3/global";
    let response: ApiResponse = reqwest::get(url).await?.json().await?;

    // 仮想通貨全体の市場統計情報を取得
    let g = response.data;

    // f64 → BigDecimal に変換
    let total_market_cap_usd = g.total_market_cap.get("usd").and_then(|v| BigDecimal::from_f64(*v));
    let total_volume_usd = g.total_volume.get("usd").and_then(|v| BigDecimal::from_f64(*v));
    let btc_dominance = g.market_cap_percentage.get("btc").and_then(|v| BigDecimal::from_f64(*v));
    let eth_dominance = g.market_cap_percentage.get("eth").and_then(|v| BigDecimal::from_f64(*v));

    // global.market_statsテーブルにデータを挿入
    sqlx::query!(
        r#"
        INSERT INTO global.market_stats (
            active_cryptocurrencies,
            upcoming_icos,
            ongoing_icos,
            ended_icos,
            markets,
            total_market_cap_usd,
            total_volume_usd,
            btc_dominance,
            eth_dominance,
            fetched_at
        )
        VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9, now())
        "#,
        g.active_cryptocurrencies,
        g.upcoming_icos,
        g.ongoing_icos,
        g.ended_icos,
        g.markets,
        total_market_cap_usd,
        total_volume_usd,
        btc_dominance,
        eth_dominance,
    )
    .execute(&pool)
    .await?;

    // 処理完了メッセージ
    println!("✅ Successfully inserted into global.market_stats!");
    Ok(())
}

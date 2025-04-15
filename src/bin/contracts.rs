// 環境変数の読み込み（.envファイル用）
use dotenv::dotenv;
// 環境変数の操作用（DATABASE_URLの取得など）
use std::env;
// PostgreSQL接続用ライブラリ
use sqlx::PgPool;
// APIレスポンスの自動デシリアライズ用
use serde::Deserialize;
// 非同期での時間待機用
use tokio::time::{sleep, Duration};

// コイン一覧APIのレスポンスで使用（IDのみ取得）
#[derive(Debug, Deserialize)]
struct Coin {
    id: String,
}

// コイン詳細APIのレスポンスで使用（必要な情報のみ保持）
#[derive(Debug, Deserialize)]
struct CoinDetail {
    name: Option<String>,
    symbol: Option<String>,
    platforms: std::collections::HashMap<String, String>,
    decimals: Option<i32>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // .envファイルを読み込み
    dotenv().ok();

    // DATABASE_URLを取得してDBに接続
    let database_url = env::var("DATABASE_URL")?;
    let pool = PgPool::connect(&database_url).await?;

    // CoinGecko APIから、全コインのIDリストを取得
    let coin_list_url = "https://api.coingecko.com/api/v3/coins/list?include_platform=true";
    let coin_list: Vec<Coin> = reqwest::get(coin_list_url).await?.json().await?;

    // 上位100件だけ処理対象とする（APIレート制限対策）
    for coin in coin_list.iter().take(100) {
        // 各コインの詳細情報を取得するためのURLを構築
        let url = format!("https://api.coingecko.com/api/v3/coins/{}", coin.id);
        let res = reqwest::get(&url).await?;

        // CoinDetail構造体に変換（JSONパース）
        match res.json::<CoinDetail>().await {
            Ok(detail) => {
                // 各プラットフォームごとに処理
                for (platform, address) in detail.platforms.iter() {
                    // コントラクトアドレスが空文字ならスキップ
                    if address.is_empty() {
                        continue;
                    }

                    // 挿入ログを表示
                    println!("📥 Inserting contract: {} on {}", address, platform);

                    // PostgreSQL にコントラクト情報をINSERT（timestampはnow()）
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
            // JSON変換に失敗した場合のエラー出力
            Err(e) => {
                println!("⚠️ Failed to parse detail for {}: {}", coin.id, e);
            }
        }

        // CoinGecko APIのレート制限回避
        sleep(Duration::from_millis(1500)).await;
    }

    // 正常終了メッセージ
    println!("✅ Successfully inserted contracts into contract.token_info!");
    Ok(())
}

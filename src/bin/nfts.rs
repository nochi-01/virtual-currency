// 環境変数を読み込むためのライブラリ
use dotenv::dotenv;
// 環境変数の取得やSQL接続に必要な標準ライブラリ
use std::env;
// PostgreSQL用の非同期接続とBigDecimal型
use sqlx::{PgPool, types::BigDecimal};
// f64 → BigDecimalへの変換のため
use num_traits::FromPrimitive;
// JSONデシリアライズ用
use serde::Deserialize;
// floor_priceやvolume_24hの連想配列に対応
use std::collections::HashMap;
// APIレート制限を避けるための待機処理
use tokio::time::{sleep, Duration};

// NFT一覧取得用の構造体（/nfts/list の1件分）
#[derive(Debug, Deserialize)]
struct NftListItem {
    id: String,
}

// 各NFTの詳細情報の構造体（/nfts/{id} のレスポンス構造）
#[derive(Debug, Deserialize)]
struct NftDetail {
    id: String,
    name: Option<String>,
    symbol: Option<String>,
    floor_price: Option<HashMap<String, f64>>,
    volume_24h: Option<HashMap<String, f64>>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // .envファイルから環境変数（DATABASE_URLなど）を読み込む
    dotenv().ok();

    // DATABASE_URLからPostgreSQLへの接続文字列を取得
    let database_url = env::var("DATABASE_URL")?;

    // PostgreSQLとの非同期接続プールを作成
    let pool = PgPool::connect(&database_url).await?;

    // CoinGecko APIからNFT一覧を取得（最大数を制限するため Vec にパース）
    let nft_list: Vec<NftListItem> = reqwest::get("https://api.coingecko.com/api/v3/nfts/list")
        .await?
        .json()
        .await?;

    // 取得したNFTの最初の10件だけ処理
    for nft in nft_list.iter().take(10) {
        // 各NFTの詳細情報APIのURLを組み立て
        let detail_url = format!("https://api.coingecko.com/api/v3/nfts/{}", nft.id);
        
        // 詳細情報を取得
        let res = reqwest::get(&detail_url).await?;

        // 正常にパースできなければスキップ
        let Ok(detail) = res.json::<NftDetail>().await else {
            println!("⚠️ Failed to parse NFT: {}", nft.id);
            continue;
        };

        // デバッグ出力（取得したNFTの詳細）
        println!("📥 Inserting NFT: {:?}", detail);

        // floor_price["usd"] を BigDecimal に変換
        let floor_price = detail.floor_price.as_ref()
            .and_then(|map| map.get("usd"))
            .and_then(|v| BigDecimal::from_f64(*v));

        // volume_24h["usd"] を BigDecimal に変換
        let volume_24h = detail.volume_24h.as_ref()
            .and_then(|map| map.get("usd"))
            .and_then(|v| BigDecimal::from_f64(*v));

        // 取得した情報を nfts.collections テーブルにINSERT
        sqlx::query!(
            r#"
            INSERT INTO nfts.collections (
                id,
                name,
                floor_price,
                volume_24h,
                symbol,
                fetched_at
            )
            VALUES ($1, $2, $3, $4, $5, now())
            "#,
            detail.id,
            detail.name,
            floor_price,
            volume_24h,
            detail.symbol
        )
        .execute(&pool)
        .await?;

        // APIレート制限対策のため、1秒間隔で次のリクエストを送る
        sleep(Duration::from_millis(1000)).await;
    }

    // 処理完了ログ
    println!("✅ Successfully inserted NFTs into nfts.collections!");
    Ok(())
}

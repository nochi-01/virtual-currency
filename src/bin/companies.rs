use dotenv::dotenv;
use std::env;
use sqlx::{PgPool, types::BigDecimal};
use num_traits::FromPrimitive;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Company {
    name: String,
    symbol: String,
    total_holdings: Option<f64>,
    total_value_usd: Option<f64>,
    percentage_of_supply: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct ApiResponse {
    companies: Vec<Company>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // .envからDATABASE_URLを取得してDB接続
    dotenv().ok();
    let database_url = env::var("DATABASE_URL")?;
    let pool = PgPool::connect(&database_url).await?;

    // CoinGecko APIから企業のBTC保有情報を取得
    let url = "https://api.coingecko.com/api/v3/companies/public_treasury/bitcoin";
    let response = reqwest::get(url).await?.json::<ApiResponse>().await?;

    // 各企業情報をDBに挿入
    for company in response.companies {
        println!("📥 Inserting company: {:?}", company.name);

        let holdings_bd = company.total_holdings.and_then(BigDecimal::from_f64);
        let value_bd = company.total_value_usd.and_then(BigDecimal::from_f64);
        let percent_bd = company.percentage_of_supply.and_then(BigDecimal::from_f64);

        sqlx::query!(
            r#"
            INSERT INTO companies.public_holdings (
                company_name,
                symbol,
                total_holdings,
                total_value_usd,
                percentage_of_supply,
                fetched_at
            )
            VALUES ($1, $2, $3, $4, $5, now())
            "#,
            company.name,
            company.symbol,
            holdings_bd,
            value_bd,
            percent_bd
        )
        .execute(&pool)
        .await?;
    }

    println!("✅ Successfully inserted companies into companies.public_holdings!");
    Ok(())
}

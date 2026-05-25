use anyhow::Context as _;
use chrono::{Local, NaiveDate};
use rustcash_core::{account::{Account, AccountType}, ids::{AccountId, BookId}};
use rustcash_engine::balance::{AccountBalance, BalanceService};
use rustcash_storage::{repositories::accounts::AccountRepository, SqlitePool};
use serde_json;

// ── data fetchers ─────────────────────────────────────────────────────────────

pub async fn list_accounts(pool: &SqlitePool, book_id: BookId) -> anyhow::Result<Vec<Account>> {
    Ok(AccountRepository::new(pool.clone()).find_by_book(book_id).await?)
}

pub async fn get_account(pool: &SqlitePool, id_str: &str) -> anyhow::Result<Account> {
    let uuid = id_str
        .parse::<uuid::Uuid>()
        .with_context(|| format!("invalid account ID: {id_str}"))?;
    let account_id = AccountId::from(uuid);
    AccountRepository::new(pool.clone())
        .find_by_id(account_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("account {id_str} not found"))
}

pub async fn get_account_balance(
    pool: &SqlitePool,
    id_str: &str,
    book_id: BookId,
    as_of: NaiveDate,
) -> anyhow::Result<AccountBalance> {
    let uuid = id_str
        .parse::<uuid::Uuid>()
        .with_context(|| format!("invalid account ID: {id_str}"))?;
    let account_id = AccountId::from(uuid);
    Ok(BalanceService::new(pool.clone())
        .account_balance(account_id, book_id, as_of)
        .await?)
}

// ── formatters ────────────────────────────────────────────────────────────────

/// Returns the serde snake_case name for an `AccountType` (e.g. `credit_card`).
pub fn account_type_str(t: AccountType) -> String {
    serde_json::to_value(t)
        .ok()
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_else(|| format!("{t:?}"))
}

pub fn render_table(accounts: &[Account]) -> String {
    let mut out = format!("{:<36}  {:<14}  {}\n", "ID", "TYPE", "FULL NAME");
    out.push_str(&format!("{}\n", "─".repeat(80)));
    for a in accounts {
        out.push_str(&format!(
            "{:<36}  {:<14}  {}\n",
            a.id,
            account_type_str(a.account_type),
            a.full_name,
        ));
    }
    out
}

pub fn render_json(accounts: &[Account]) -> anyhow::Result<String> {
    Ok(serde_json::to_string_pretty(accounts)?)
}

pub fn render_csv(accounts: &[Account]) -> String {
    let mut out = "id,type,full_name,name,placeholder,hidden\n".to_string();
    for a in accounts {
        out.push_str(&format!(
            "{},{},{},{},{},{}\n",
            a.id,
            account_type_str(a.account_type),
            a.full_name,
            a.name,
            a.placeholder,
            a.hidden,
        ));
    }
    out
}

pub fn render_detail(account: &Account) -> String {
    let mut out = String::new();
    out.push_str(&format!("{:<14} {}\n", "ID:",        account.id));
    out.push_str(&format!("{:<14} {}\n", "Name:",      account.name));
    out.push_str(&format!("{:<14} {}\n", "Full name:", account.full_name));
    out.push_str(&format!("{:<14} {}\n", "Type:",      account_type_str(account.account_type)));
    out.push_str(&format!("{:<14} {}\n", "Commodity:", account.commodity_id));
    if let Some(desc) = &account.description {
        out.push_str(&format!("{:<14} {}\n", "Description:", desc));
    }
    out.push_str(&format!("{:<14} {}\n", "Placeholder:", account.placeholder));
    out.push_str(&format!("{:<14} {}\n", "Hidden:",    account.hidden));
    out.push_str(&format!("{:<14} {}\n", "Created:",   account.created_at.format("%Y-%m-%d %H:%M UTC")));
    out
}

pub fn render_balance(account: &Account, balance: &AccountBalance) -> String {
    let mut out = String::new();
    out.push_str(&format!("{:<14} {}\n", "Account:",    account.full_name));
    out.push_str(&format!("{:<14} {}\n", "As of:",      balance.as_of));
    out.push_str(&format!("{:<14} {}\n", "Balance:",    balance.balance));
    out.push_str(&format!("{:<14} {}\n", "Cleared:",    balance.cleared_balance));
    out.push_str(&format!("{:<14} {}\n", "Reconciled:", balance.reconciled_balance));
    out
}

// ── CLI entry points ──────────────────────────────────────────────────────────

pub async fn cmd_list(pool: &SqlitePool, book_id: BookId, format: &str) -> anyhow::Result<()> {
    let accounts = list_accounts(pool, book_id).await?;
    let output = match format {
        "json" => render_json(&accounts)?,
        "csv"  => render_csv(&accounts),
        _      => render_table(&accounts),
    };
    print!("{output}");
    Ok(())
}

pub async fn cmd_show(pool: &SqlitePool, id: &str) -> anyhow::Result<()> {
    let account = get_account(pool, id).await?;
    print!("{}", render_detail(&account));
    Ok(())
}

pub async fn cmd_balance(
    pool: &SqlitePool,
    id: &str,
    book_id: BookId,
    as_of_str: Option<&str>,
) -> anyhow::Result<()> {
    let as_of = match as_of_str {
        Some(s) => NaiveDate::parse_from_str(s, "%Y-%m-%d")
            .with_context(|| format!("invalid date '{s}': expected YYYY-MM-DD"))?,
        None => Local::now().date_naive(),
    };
    let account = get_account(pool, id).await?;
    let balance = get_account_balance(pool, id, book_id, as_of).await?;
    print!("{}", render_balance(&account, &balance));
    Ok(())
}

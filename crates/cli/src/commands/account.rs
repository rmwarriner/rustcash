use anyhow::Context as _;
use chrono::{Local, NaiveDate, Utc};
use rustcash_core::{
    account::{Account, AccountType},
    ids::{AccountId, BookId, CommodityId},
};
use rustcash_engine::{
    account::AccountService,
    balance::{AccountBalance, BalanceService},
};
use rustcash_storage::{
    SqlitePool,
    repositories::{accounts::AccountRepository, books::BookRepository},
};
use serde_json;

// ── data fetchers ─────────────────────────────────────────────────────────────

pub async fn list_accounts(pool: &SqlitePool, book_id: BookId) -> anyhow::Result<Vec<Account>> {
    Ok(AccountRepository::new(pool.clone())
        .find_by_book(book_id)
        .await?)
}

/// Resolve an account by UUID string or full hierarchical name (e.g. `"Assets:Checking"`).
/// UUID is tried first; if the string doesn't parse as one it is treated as a full_name.
pub async fn get_account(
    pool: &SqlitePool,
    id_or_name: &str,
    book_id: BookId,
) -> anyhow::Result<Account> {
    if let Ok(uuid) = id_or_name.parse::<uuid::Uuid>() {
        return AccountRepository::new(pool.clone())
            .find_by_id(AccountId::from(uuid))
            .await?
            .ok_or_else(|| anyhow::anyhow!("account '{id_or_name}' not found"));
    }
    AccountRepository::new(pool.clone())
        .find_by_full_name(book_id, id_or_name)
        .await?
        .ok_or_else(|| anyhow::anyhow!("account '{id_or_name}' not found"))
}

pub async fn get_account_balance(
    pool: &SqlitePool,
    id_or_name: &str,
    book_id: BookId,
    as_of: NaiveDate,
) -> anyhow::Result<AccountBalance> {
    let account = get_account(pool, id_or_name, book_id).await?;
    Ok(BalanceService::new(pool.clone())
        .account_balance(account.id, book_id, as_of)
        .await?)
}

// ── account type parsing ──────────────────────────────────────────────────────

/// Parse a snake_case account type string (e.g. `"credit_card"`) into `AccountType`.
pub fn parse_account_type(s: &str) -> anyhow::Result<AccountType> {
    serde_json::from_value(serde_json::Value::String(s.to_string()))
        .with_context(|| format!("unknown account type '{s}'; valid types: asset, cash, bank, credit_card, investment, mutual_fund, liability, long_term_liability, equity, opening_balance, retained_earnings, income, expense, receivable, payable"))
}

// ── write operations ──────────────────────────────────────────────────────────

pub struct CreateAccountArgs<'a> {
    pub name: &'a str,
    pub type_str: &'a str,
    pub parent_id: Option<&'a str>,
    pub commodity_id: Option<&'a str>,
    pub description: Option<&'a str>,
    pub placeholder: bool,
    pub hidden: bool,
}

pub async fn cmd_create(
    pool: &SqlitePool,
    book_id: BookId,
    args: CreateAccountArgs<'_>,
) -> anyhow::Result<()> {
    let CreateAccountArgs {
        name,
        type_str,
        parent_id,
        commodity_id,
        description,
        placeholder,
        hidden,
    } = args;
    let account_type = parse_account_type(type_str)?;

    // Resolve commodity: use supplied ID or fall back to book default.
    let commodity_id = if let Some(s) = commodity_id {
        CommodityId::from(
            s.parse::<uuid::Uuid>()
                .with_context(|| format!("invalid commodity ID: {s}"))?,
        )
    } else {
        BookRepository::new(pool.clone())
            .find_by_id(book_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("book {book_id} not found"))?
            .default_commodity_id
    };

    // Resolve optional parent by UUID or full_name; build child's full_name.
    let (parent_id, full_name) = if let Some(p) = parent_id {
        let parent = get_account(pool, p, book_id).await?;
        let full = format!("{}:{}", parent.full_name, name);
        (Some(parent.id), full)
    } else {
        (None, name.to_string())
    };

    let account = Account {
        id: AccountId::new(),
        book_id,
        parent_id,
        name: name.to_string(),
        full_name,
        account_type,
        commodity_id,
        description: description.map(str::to_string),
        placeholder,
        hidden,
        sort_order: 0,
        created_at: Utc::now(),
        modified_at: Utc::now(),
        deleted_at: None,
    };

    AccountService::new(pool.clone()).create(&account).await?;

    println!("Created account {} ({})", account.full_name, account.id);
    Ok(())
}

pub async fn cmd_rename(
    pool: &SqlitePool,
    book_id: BookId,
    id_or_name: &str,
    name: &str,
) -> anyhow::Result<()> {
    let account = get_account(pool, id_or_name, book_id).await?;

    let new_full_name = if let Some(pid) = account.parent_id {
        let parent = AccountRepository::new(pool.clone())
            .find_by_id(pid)
            .await?
            .ok_or_else(|| anyhow::anyhow!("parent account not found"))?;
        format!("{}:{}", parent.full_name, name)
    } else {
        name.to_string()
    };

    AccountService::new(pool.clone())
        .rename(account.id, name.to_string(), new_full_name.clone())
        .await?;

    println!("Renamed to {} ({})", new_full_name, account.id);
    Ok(())
}

pub async fn cmd_delete(
    pool: &SqlitePool,
    book_id: BookId,
    id_or_name: &str,
) -> anyhow::Result<()> {
    let account = get_account(pool, id_or_name, book_id).await?;

    AccountService::new(pool.clone())
        .soft_delete(account.id)
        .await?;

    println!("Deleted account {} ({})", account.full_name, account.id);
    Ok(())
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
    out.push_str(&format!("{:<14} {}\n", "ID:", account.id));
    out.push_str(&format!("{:<14} {}\n", "Name:", account.name));
    out.push_str(&format!("{:<14} {}\n", "Full name:", account.full_name));
    out.push_str(&format!(
        "{:<14} {}\n",
        "Type:",
        account_type_str(account.account_type)
    ));
    out.push_str(&format!("{:<14} {}\n", "Commodity:", account.commodity_id));
    if let Some(desc) = &account.description {
        out.push_str(&format!("{:<14} {}\n", "Description:", desc));
    }
    out.push_str(&format!("{:<14} {}\n", "Placeholder:", account.placeholder));
    out.push_str(&format!("{:<14} {}\n", "Hidden:", account.hidden));
    out.push_str(&format!(
        "{:<14} {}\n",
        "Created:",
        account.created_at.format("%Y-%m-%d %H:%M UTC")
    ));
    out
}

pub fn render_balance(account: &Account, balance: &AccountBalance) -> String {
    let mut out = String::new();
    out.push_str(&format!("{:<14} {}\n", "Account:", account.full_name));
    out.push_str(&format!("{:<14} {}\n", "As of:", balance.as_of));
    out.push_str(&format!("{:<14} {}\n", "Balance:", balance.balance));
    out.push_str(&format!("{:<14} {}\n", "Cleared:", balance.cleared_balance));
    out.push_str(&format!(
        "{:<14} {}\n",
        "Reconciled:", balance.reconciled_balance
    ));
    out
}

// ── CLI entry points ──────────────────────────────────────────────────────────

pub async fn cmd_list(pool: &SqlitePool, book_id: BookId, format: &str) -> anyhow::Result<()> {
    let accounts = list_accounts(pool, book_id).await?;
    let output = match format {
        "json" => render_json(&accounts)?,
        "csv" => render_csv(&accounts),
        _ => render_table(&accounts),
    };
    print!("{output}");
    Ok(())
}

pub async fn cmd_show(pool: &SqlitePool, book_id: BookId, id_or_name: &str) -> anyhow::Result<()> {
    let account = get_account(pool, id_or_name, book_id).await?;
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
    let account = get_account(pool, id, book_id).await?;
    let balance = get_account_balance(pool, id, book_id, as_of).await?;
    print!("{}", render_balance(&account, &balance));
    Ok(())
}

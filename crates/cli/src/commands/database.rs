use anyhow::Context as _;
use chrono::Utc;
use rustcash_core::{
    book::Book,
    commodity::Commodity,
    ids::{BookId, CommodityId},
};
use rustcash_storage::{
    SqlitePool, open_sqlite,
    repositories::{
        accounts::AccountRepository, books::BookRepository, commodities::CommodityRepository,
    },
    run_migrations,
};

// ── currency helpers ──────────────────────────────────────────────────────────

/// Return `(display_name, fraction)` for a known currency code, or fall back
/// to `(code.to_uppercase(), 100)` for anything unrecognised.
pub fn currency_info(code: &str) -> (String, u32) {
    match code.to_uppercase().as_str() {
        "USD" => ("US Dollar".into(), 100),
        "EUR" => ("Euro".into(), 100),
        "GBP" => ("Pound Sterling".into(), 100),
        "CAD" => ("Canadian Dollar".into(), 100),
        "AUD" => ("Australian Dollar".into(), 100),
        "NZD" => ("New Zealand Dollar".into(), 100),
        "CHF" => ("Swiss Franc".into(), 100),
        "JPY" => ("Japanese Yen".into(), 1),
        "CNY" => ("Chinese Yuan".into(), 10),
        "INR" => ("Indian Rupee".into(), 100),
        "BRL" => ("Brazilian Real".into(), 100),
        "MXN" => ("Mexican Peso".into(), 100),
        "SEK" => ("Swedish Krona".into(), 100),
        "NOK" => ("Norwegian Krone".into(), 100),
        "DKK" => ("Danish Krone".into(), 100),
        "SGD" => ("Singapore Dollar".into(), 100),
        "HKD" => ("Hong Kong Dollar".into(), 100),
        other => (other.to_string(), 100),
    }
}

// ── core logic (testable) ─────────────────────────────────────────────────────

/// Insert a commodity and a book into `pool`.
/// Returns `(Book, Commodity)` on success.
/// Does not check for existing books — that guard lives in `cmd_init`.
pub async fn create_book_in_pool(
    pool: &SqlitePool,
    name: &str,
    currency: &str,
) -> anyhow::Result<(Book, Commodity)> {
    let (currency_name, fraction) = currency_info(currency);
    let mnemonic = currency.to_uppercase();

    // Pre-assign both IDs so we can insert book → commodity in FK order.
    // `books.default_commodity_id` has no FK constraint, so referencing a
    // not-yet-inserted commodity ID is safe. `commodities.book_id` does have
    // a FK constraint, so the book must exist first.
    let book_id = BookId::new();
    let commodity_id = CommodityId::new();

    let book = Book {
        id: book_id,
        name: name.into(),
        description: None,
        default_commodity_id: commodity_id,
        period_close_date: None,
        owner_id: None,
        created_at: Utc::now(),
        modified_at: Utc::now(),
        deleted_at: None,
    };
    BookRepository::new(pool.clone()).insert(&book).await?;

    let commodity = Commodity {
        id: commodity_id,
        book_id,
        namespace: "CURRENCY".into(),
        mnemonic,
        name: currency_name,
        fraction,
        notes: None,
        created_at: Utc::now(),
    };
    CommodityRepository::new(pool.clone())
        .insert(&commodity)
        .await?;

    Ok((book, commodity))
}

// ── CLI entry points ──────────────────────────────────────────────────────────

pub async fn cmd_init(
    db_path: &str,
    name: &str,
    currency: &str,
    force: bool,
) -> anyhow::Result<()> {
    // Create the parent directory if it doesn't exist yet.
    let fs_path = std::path::Path::new(db_path);
    if let Some(parent) = fs_path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating directory {}", parent.display()))?;
        }
    }

    let url = db_url(db_path);
    let pool = open_sqlite(&url).await.context("opening database")?;
    run_migrations(&pool).await.context("running migrations")?;

    if !force {
        let existing = BookRepository::new(pool.clone()).find_all().await?;
        if !existing.is_empty() {
            anyhow::bail!(
                "database already contains {} book(s)\n\
                 Use --force to create an additional book alongside existing ones.",
                existing.len()
            );
        }
    }

    let (book, commodity) = create_book_in_pool(&pool, name, currency).await?;

    println!("Initialized: {db_path}");
    println!("Book:        {} ({})", book.name, book.id);
    println!("Currency:    {} — {}", commodity.mnemonic, commodity.name);

    Ok(())
}

pub async fn cmd_status(db_path: &str) -> anyhow::Result<()> {
    let fs_path = std::path::Path::new(db_path);
    if !fs_path.exists() {
        println!("Not initialized: {db_path}");
        println!("Run `rustcash database init` to create it.");
        return Ok(());
    }

    let url = db_url(db_path);
    let pool = open_sqlite(&url).await.context("opening database")?;
    run_migrations(&pool).await.context("running migrations")?;

    let books = BookRepository::new(pool.clone()).find_all().await?;
    println!("Database: {db_path}");
    println!("Books:    {}", books.len());
    for book in &books {
        let accounts = AccountRepository::new(pool.clone())
            .find_by_book(book.id)
            .await?;
        println!(
            "  {} ({}) — {} accounts",
            book.name,
            book.id,
            accounts.len()
        );
    }

    Ok(())
}

pub async fn cmd_backup(_db_path: &str, _output: Option<&str>) -> anyhow::Result<()> {
    println!("database backup — not yet implemented");
    Ok(())
}

pub async fn cmd_seed(
    _db_path: &str,
    _book_id: Option<&str>,
    _template: &str,
) -> anyhow::Result<()> {
    println!("database seed — not yet implemented");
    Ok(())
}

pub async fn cmd_purge(
    _db_path: &str,
    _older_than_days: u32,
    _dry_run: bool,
) -> anyhow::Result<()> {
    println!("database purge — not yet implemented");
    Ok(())
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn db_url(path: &str) -> String {
    if path.starts_with("sqlite:") {
        path.to_string()
    } else {
        format!("sqlite:{path}")
    }
}

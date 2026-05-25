use rustcash_cli::commands::database::{create_book_in_pool, currency_info};
use rustcash_storage::{SqlitePool, repositories::books::BookRepository};

// ── currency_info ─────────────────────────────────────────────────────────────

#[test]
fn known_currency_returns_full_name() {
    let (name, fraction) = currency_info("USD");
    assert_eq!(name, "US Dollar");
    assert_eq!(fraction, 100);
}

#[test]
fn known_currency_case_insensitive() {
    let (name, _) = currency_info("eur");
    assert_eq!(name, "Euro");
}

#[test]
fn unknown_currency_falls_back_to_code() {
    let (name, fraction) = currency_info("XYZ");
    assert_eq!(name, "XYZ");
    assert_eq!(fraction, 100);
}

// ── create_book_in_pool ───────────────────────────────────────────────────────

#[sqlx::test(migrations = "../storage/migrations")]
async fn init_creates_book_and_commodity(pool: SqlitePool) {
    let (book, commodity) = create_book_in_pool(&pool, "My Finances", "USD")
        .await
        .unwrap();
    assert_eq!(book.name, "My Finances");
    assert_eq!(commodity.mnemonic, "USD");
    assert_eq!(commodity.name, "US Dollar");
    assert_eq!(commodity.fraction, 100);
    assert_eq!(commodity.namespace, "CURRENCY");
    assert_eq!(book.default_commodity_id, commodity.id);
}

#[sqlx::test(migrations = "../storage/migrations")]
async fn init_book_is_persisted(pool: SqlitePool) {
    let (book, _) = create_book_in_pool(&pool, "Test Book", "EUR")
        .await
        .unwrap();

    let found = BookRepository::new(pool.clone())
        .find_by_id(book.id)
        .await
        .unwrap()
        .expect("book should exist in DB");
    assert_eq!(found.name, "Test Book");
}

#[sqlx::test(migrations = "../storage/migrations")]
async fn init_currency_is_uppercased(pool: SqlitePool) {
    let (_, commodity) = create_book_in_pool(&pool, "Book", "gbp").await.unwrap();
    assert_eq!(commodity.mnemonic, "GBP");
}

#[sqlx::test(migrations = "../storage/migrations")]
async fn init_can_create_multiple_books(pool: SqlitePool) {
    create_book_in_pool(&pool, "Book A", "USD").await.unwrap();
    create_book_in_pool(&pool, "Book B", "EUR").await.unwrap();

    let books = BookRepository::new(pool.clone()).find_all().await.unwrap();
    assert_eq!(books.len(), 2);
}

use chrono::{NaiveDate, Utc};
use rustcash_cli::commands::account::{
    account_type_str, get_account, get_account_balance, list_accounts, render_csv, render_detail,
    render_json, render_table,
};
use rustcash_core::{
    account::{Account, AccountType},
    book::Book,
    commodity::Commodity,
    ids::{AccountId, BookId, CommodityId},
};
use rustcash_storage::{
    repositories::{
        accounts::AccountRepository, books::BookRepository, commodities::CommodityRepository,
    },
    SqlitePool,
};

// ── fixtures ──────────────────────────────────────────────────────────────────

async fn insert_book(pool: &SqlitePool) -> Book {
    let commodity_id = CommodityId::new();
    let book = Book {
        id:                   BookId::new(),
        name:                 "Test Book".into(),
        description:          None,
        default_commodity_id: commodity_id,
        period_close_date:    None,
        owner_id:             None,
        created_at:           Utc::now(),
        modified_at:          Utc::now(),
        deleted_at:           None,
    };
    BookRepository::new(pool.clone()).insert(&book).await.unwrap();
    book
}

async fn insert_commodity(pool: &SqlitePool, book_id: BookId) -> Commodity {
    let c = Commodity {
        id:         CommodityId::new(),
        book_id,
        namespace:  "CURRENCY".into(),
        mnemonic:   "USD".into(),
        name:       "US Dollar".into(),
        fraction:   100,
        notes:      None,
        created_at: Utc::now(),
    };
    CommodityRepository::new(pool.clone()).insert(&c).await.unwrap();
    c
}

fn make_account(book_id: BookId, commodity_id: CommodityId, name: &str, full_name: &str, account_type: AccountType) -> Account {
    Account {
        id:           AccountId::new(),
        book_id,
        parent_id:    None,
        name:         name.into(),
        full_name:    full_name.into(),
        account_type,
        commodity_id,
        description:  None,
        placeholder:  false,
        hidden:       false,
        sort_order:   0,
        created_at:   Utc::now(),
        modified_at:  Utc::now(),
        deleted_at:   None,
    }
}

// ── list_accounts ─────────────────────────────────────────────────────────────

#[sqlx::test(migrations = "../storage/migrations")]
async fn list_returns_all_active_accounts(pool: SqlitePool) {
    let book = insert_book(&pool).await;
    let commodity = insert_commodity(&pool, book.id).await;
    let repo = AccountRepository::new(pool.clone());

    let checking = make_account(book.id, commodity.id, "Checking", "Assets:Checking", AccountType::Bank);
    let savings  = make_account(book.id, commodity.id, "Savings",  "Assets:Savings",  AccountType::Bank);
    repo.insert(&checking).await.unwrap();
    repo.insert(&savings).await.unwrap();

    let accounts = list_accounts(&pool, book.id).await.unwrap();
    assert_eq!(accounts.len(), 2);
    let full_names: Vec<&str> = accounts.iter().map(|a| a.full_name.as_str()).collect();
    assert!(full_names.contains(&"Assets:Checking"));
    assert!(full_names.contains(&"Assets:Savings"));
}

#[sqlx::test(migrations = "../storage/migrations")]
async fn list_excludes_deleted_accounts(pool: SqlitePool) {
    let book = insert_book(&pool).await;
    let commodity = insert_commodity(&pool, book.id).await;
    let repo = AccountRepository::new(pool.clone());

    let active  = make_account(book.id, commodity.id, "Active",  "Assets:Active",  AccountType::Asset);
    let mut deleted = make_account(book.id, commodity.id, "Gone", "Assets:Gone", AccountType::Asset);
    deleted.deleted_at = Some(Utc::now());

    repo.insert(&active).await.unwrap();
    repo.insert(&deleted).await.unwrap();

    let accounts = list_accounts(&pool, book.id).await.unwrap();
    assert_eq!(accounts.len(), 1);
    assert_eq!(accounts[0].name, "Active");
}

// ── get_account ───────────────────────────────────────────────────────────────

#[sqlx::test(migrations = "../storage/migrations")]
async fn get_account_finds_by_id(pool: SqlitePool) {
    let book = insert_book(&pool).await;
    let commodity = insert_commodity(&pool, book.id).await;
    let repo = AccountRepository::new(pool.clone());

    let acct = make_account(book.id, commodity.id, "Checking", "Assets:Checking", AccountType::Bank);
    repo.insert(&acct).await.unwrap();

    let found = get_account(&pool, &acct.id.to_string()).await.unwrap();
    assert_eq!(found.id, acct.id);
    assert_eq!(found.full_name, "Assets:Checking");
}

#[sqlx::test(migrations = "../storage/migrations")]
async fn get_account_errors_on_bad_id(pool: SqlitePool) {
    let err = get_account(&pool, "not-a-uuid").await;
    assert!(err.is_err());
    assert!(err.unwrap_err().to_string().contains("invalid account ID"));
}

#[sqlx::test(migrations = "../storage/migrations")]
async fn get_account_errors_when_not_found(pool: SqlitePool) {
    let missing_id = AccountId::new().to_string();
    let err = get_account(&pool, &missing_id).await;
    assert!(err.is_err());
    assert!(err.unwrap_err().to_string().contains("not found"));
}

// ── get_account_balance ───────────────────────────────────────────────────────

#[sqlx::test(migrations = "../storage/migrations")]
async fn balance_is_zero_for_account_with_no_transactions(pool: SqlitePool) {
    let book = insert_book(&pool).await;
    let commodity = insert_commodity(&pool, book.id).await;
    let repo = AccountRepository::new(pool.clone());

    let acct = make_account(book.id, commodity.id, "Checking", "Assets:Checking", AccountType::Bank);
    repo.insert(&acct).await.unwrap();

    let as_of = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
    let balance = get_account_balance(&pool, &acct.id.to_string(), book.id, as_of).await.unwrap();
    assert_eq!(balance.balance, rust_decimal::Decimal::ZERO);
    assert_eq!(balance.account_id, acct.id);
}

// ── render functions ──────────────────────────────────────────────────────────

#[test]
fn render_table_has_header_and_row() {
    let acct = Account {
        id:           AccountId::new(),
        book_id:      BookId::new(),
        parent_id:    None,
        name:         "Checking".into(),
        full_name:    "Assets:Checking".into(),
        account_type: AccountType::Bank,
        commodity_id: CommodityId::new(),
        description:  None,
        placeholder:  false,
        hidden:       false,
        sort_order:   0,
        created_at:   Utc::now(),
        modified_at:  Utc::now(),
        deleted_at:   None,
    };
    let output = render_table(&[acct]);
    assert!(output.contains("FULL NAME"));
    assert!(output.contains("Assets:Checking"));
    assert!(output.contains("bank"));
}

#[test]
fn render_json_is_valid_array() {
    let output = render_json(&[]).unwrap();
    assert_eq!(output.trim(), "[]");
}

#[test]
fn render_csv_has_header() {
    let output = render_csv(&[]);
    assert!(output.starts_with("id,type,full_name"));
}

#[test]
fn account_type_str_is_snake_case() {
    assert_eq!(account_type_str(AccountType::Bank), "bank");
    assert_eq!(account_type_str(AccountType::CreditCard), "credit_card");
    assert_eq!(account_type_str(AccountType::MutualFund), "mutual_fund");
}

#[test]
fn render_detail_shows_key_fields() {
    let acct = Account {
        id:           AccountId::new(),
        book_id:      BookId::new(),
        parent_id:    None,
        name:         "Checking".into(),
        full_name:    "Assets:Checking".into(),
        account_type: AccountType::Bank,
        commodity_id: CommodityId::new(),
        description:  Some("My main account".into()),
        placeholder:  false,
        hidden:       false,
        sort_order:   0,
        created_at:   Utc::now(),
        modified_at:  Utc::now(),
        deleted_at:   None,
    };
    let output = render_detail(&acct);
    assert!(output.contains("Assets:Checking"));
    assert!(output.contains("bank"));
    assert!(output.contains("My main account"));
}

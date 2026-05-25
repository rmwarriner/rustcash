use chrono::{NaiveDate, Utc};
use rustcash_cli::commands::account::{
    CreateAccountArgs, account_type_str, cmd_create, cmd_delete, cmd_rename, get_account,
    get_account_balance, list_accounts, parse_account_type, render_csv, render_detail, render_json,
    render_table,
};
use rustcash_core::{
    account::{Account, AccountType},
    book::Book,
    commodity::Commodity,
    ids::{AccountId, BookId, CommodityId},
};
use rustcash_storage::{
    SqlitePool,
    repositories::{
        accounts::AccountRepository, books::BookRepository, commodities::CommodityRepository,
    },
};

// ── fixtures ──────────────────────────────────────────────────────────────────

async fn insert_book(pool: &SqlitePool) -> Book {
    let commodity_id = CommodityId::new();
    let book = Book {
        id: BookId::new(),
        name: "Test Book".into(),
        description: None,
        default_commodity_id: commodity_id,
        period_close_date: None,
        owner_id: None,
        created_at: Utc::now(),
        modified_at: Utc::now(),
        deleted_at: None,
    };
    BookRepository::new(pool.clone())
        .insert(&book)
        .await
        .unwrap();
    book
}

/// Create a book whose `default_commodity_id` points to a real commodity row.
/// Use this fixture when the test calls `cmd_create` without an explicit currency.
async fn insert_book_with_commodity(pool: &SqlitePool) -> (Book, Commodity) {
    let book_id = BookId::new();
    let commodity_id = CommodityId::new();
    let commodity = Commodity {
        id: commodity_id,
        book_id,
        namespace: "CURRENCY".into(),
        mnemonic: "USD".into(),
        name: "US Dollar".into(),
        fraction: 100,
        notes: None,
        created_at: Utc::now(),
    };
    let book = Book {
        id: book_id,
        name: "Test Book".into(),
        description: None,
        default_commodity_id: commodity_id,
        period_close_date: None,
        owner_id: None,
        created_at: Utc::now(),
        modified_at: Utc::now(),
        deleted_at: None,
    };
    BookRepository::new(pool.clone())
        .insert(&book)
        .await
        .unwrap();
    CommodityRepository::new(pool.clone())
        .insert(&commodity)
        .await
        .unwrap();
    (book, commodity)
}

async fn insert_commodity(pool: &SqlitePool, book_id: BookId) -> Commodity {
    let c = Commodity {
        id: CommodityId::new(),
        book_id,
        namespace: "CURRENCY".into(),
        mnemonic: "USD".into(),
        name: "US Dollar".into(),
        fraction: 100,
        notes: None,
        created_at: Utc::now(),
    };
    CommodityRepository::new(pool.clone())
        .insert(&c)
        .await
        .unwrap();
    c
}

fn make_account(
    book_id: BookId,
    commodity_id: CommodityId,
    name: &str,
    full_name: &str,
    account_type: AccountType,
) -> Account {
    Account {
        id: AccountId::new(),
        book_id,
        parent_id: None,
        name: name.into(),
        full_name: full_name.into(),
        account_type,
        commodity_id,
        description: None,
        placeholder: false,
        hidden: false,
        sort_order: 0,
        created_at: Utc::now(),
        modified_at: Utc::now(),
        deleted_at: None,
    }
}

// ── list_accounts ─────────────────────────────────────────────────────────────

#[sqlx::test(migrations = "../storage/migrations")]
async fn list_returns_all_active_accounts(pool: SqlitePool) {
    let book = insert_book(&pool).await;
    let commodity = insert_commodity(&pool, book.id).await;
    let repo = AccountRepository::new(pool.clone());

    let checking = make_account(
        book.id,
        commodity.id,
        "Checking",
        "Assets:Checking",
        AccountType::Bank,
    );
    let savings = make_account(
        book.id,
        commodity.id,
        "Savings",
        "Assets:Savings",
        AccountType::Bank,
    );
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

    let active = make_account(
        book.id,
        commodity.id,
        "Active",
        "Assets:Active",
        AccountType::Asset,
    );
    let mut deleted = make_account(
        book.id,
        commodity.id,
        "Gone",
        "Assets:Gone",
        AccountType::Asset,
    );
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

    let acct = make_account(
        book.id,
        commodity.id,
        "Checking",
        "Assets:Checking",
        AccountType::Bank,
    );
    repo.insert(&acct).await.unwrap();

    let found = get_account(&pool, &acct.id.to_string(), book.id)
        .await
        .unwrap();
    assert_eq!(found.id, acct.id);
    assert_eq!(found.full_name, "Assets:Checking");
}

#[sqlx::test(migrations = "../storage/migrations")]
async fn get_account_finds_by_full_name(pool: SqlitePool) {
    let book = insert_book(&pool).await;
    let commodity = insert_commodity(&pool, book.id).await;
    let repo = AccountRepository::new(pool.clone());

    let acct = make_account(
        book.id,
        commodity.id,
        "Checking",
        "Assets:Checking",
        AccountType::Bank,
    );
    repo.insert(&acct).await.unwrap();

    let found = get_account(&pool, "Assets:Checking", book.id)
        .await
        .unwrap();
    assert_eq!(found.id, acct.id);
}

#[sqlx::test(migrations = "../storage/migrations")]
async fn get_account_finds_root_by_name(pool: SqlitePool) {
    let book = insert_book(&pool).await;
    let commodity = insert_commodity(&pool, book.id).await;
    let repo = AccountRepository::new(pool.clone());

    let acct = make_account(
        book.id,
        commodity.id,
        "Assets",
        "Assets",
        AccountType::Asset,
    );
    repo.insert(&acct).await.unwrap();

    let found = get_account(&pool, "Assets", book.id).await.unwrap();
    assert_eq!(found.id, acct.id);
}

#[sqlx::test(migrations = "../storage/migrations")]
async fn get_account_errors_when_not_found_by_id(pool: SqlitePool) {
    let book = insert_book(&pool).await;
    let missing_id = AccountId::new().to_string();
    let err = get_account(&pool, &missing_id, book.id).await;
    assert!(err.is_err());
    assert!(err.unwrap_err().to_string().contains("not found"));
}

#[sqlx::test(migrations = "../storage/migrations")]
async fn get_account_errors_when_not_found_by_name(pool: SqlitePool) {
    let book = insert_book(&pool).await;
    let err = get_account(&pool, "Assets:Nonexistent", book.id).await;
    assert!(err.is_err());
    assert!(err.unwrap_err().to_string().contains("not found"));
}

// ── get_account_balance ───────────────────────────────────────────────────────

#[sqlx::test(migrations = "../storage/migrations")]
async fn balance_is_zero_for_account_with_no_transactions(pool: SqlitePool) {
    let book = insert_book(&pool).await;
    let commodity = insert_commodity(&pool, book.id).await;
    let repo = AccountRepository::new(pool.clone());

    let acct = make_account(
        book.id,
        commodity.id,
        "Checking",
        "Assets:Checking",
        AccountType::Bank,
    );
    repo.insert(&acct).await.unwrap();

    let as_of = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
    let balance = get_account_balance(&pool, &acct.id.to_string(), book.id, as_of)
        .await
        .unwrap();
    assert_eq!(balance.balance, rust_decimal::Decimal::ZERO);
    assert_eq!(balance.account_id, acct.id);
}

// ── render functions ──────────────────────────────────────────────────────────

#[test]
fn render_table_has_header_and_row() {
    let acct = Account {
        id: AccountId::new(),
        book_id: BookId::new(),
        parent_id: None,
        name: "Checking".into(),
        full_name: "Assets:Checking".into(),
        account_type: AccountType::Bank,
        commodity_id: CommodityId::new(),
        description: None,
        placeholder: false,
        hidden: false,
        sort_order: 0,
        created_at: Utc::now(),
        modified_at: Utc::now(),
        deleted_at: None,
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
        id: AccountId::new(),
        book_id: BookId::new(),
        parent_id: None,
        name: "Checking".into(),
        full_name: "Assets:Checking".into(),
        account_type: AccountType::Bank,
        commodity_id: CommodityId::new(),
        description: Some("My main account".into()),
        placeholder: false,
        hidden: false,
        sort_order: 0,
        created_at: Utc::now(),
        modified_at: Utc::now(),
        deleted_at: None,
    };
    let output = render_detail(&acct);
    assert!(output.contains("Assets:Checking"));
    assert!(output.contains("bank"));
    assert!(output.contains("My main account"));
}

// ── parse_account_type ────────────────────────────────────────────────────────

#[test]
fn parse_account_type_known_values() {
    assert_eq!(parse_account_type("bank").unwrap(), AccountType::Bank);
    assert_eq!(
        parse_account_type("credit_card").unwrap(),
        AccountType::CreditCard
    );
    assert_eq!(parse_account_type("expense").unwrap(), AccountType::Expense);
    assert_eq!(parse_account_type("income").unwrap(), AccountType::Income);
}

#[test]
fn parse_account_type_rejects_unknown() {
    assert!(parse_account_type("savings").is_err());
    assert!(parse_account_type("").is_err());
}

// ── cmd_create ────────────────────────────────────────────────────────────────

#[sqlx::test(migrations = "../storage/migrations")]
async fn create_root_account(pool: SqlitePool) {
    let book = insert_book(&pool).await;
    let commodity = insert_commodity(&pool, book.id).await;

    cmd_create(
        &pool,
        book.id,
        CreateAccountArgs {
            name: "Assets",
            type_str: "asset",
            parent_id: None,
            commodity_id: Some(commodity.id.to_string().as_str()),
            description: None,
            placeholder: false,
            hidden: false,
        },
    )
    .await
    .unwrap();

    let accounts = list_accounts(&pool, book.id).await.unwrap();
    assert_eq!(accounts.len(), 1);
    assert_eq!(accounts[0].name, "Assets");
    assert_eq!(accounts[0].full_name, "Assets");
}

#[sqlx::test(migrations = "../storage/migrations")]
async fn create_child_inherits_full_name(pool: SqlitePool) {
    let (book, commodity) = insert_book_with_commodity(&pool).await;

    cmd_create(
        &pool,
        book.id,
        CreateAccountArgs {
            name: "Assets",
            type_str: "asset",
            parent_id: None,
            commodity_id: Some(commodity.id.to_string().as_str()),
            description: None,
            placeholder: false,
            hidden: false,
        },
    )
    .await
    .unwrap();

    let parent = list_accounts(&pool, book.id)
        .await
        .unwrap()
        .into_iter()
        .next()
        .unwrap();

    cmd_create(
        &pool,
        book.id,
        CreateAccountArgs {
            name: "Checking",
            type_str: "bank",
            parent_id: Some(parent.id.to_string().as_str()),
            commodity_id: None,
            description: None,
            placeholder: false,
            hidden: false,
        },
    )
    .await
    .unwrap();

    let accounts = list_accounts(&pool, book.id).await.unwrap();
    let child = accounts.iter().find(|a| a.name == "Checking").unwrap();
    assert_eq!(child.full_name, "Assets:Checking");
    assert_eq!(child.parent_id, Some(parent.id));
}

#[sqlx::test(migrations = "../storage/migrations")]
async fn create_uses_book_default_commodity_when_none_given(pool: SqlitePool) {
    let (book, commodity) = insert_book_with_commodity(&pool).await;

    cmd_create(
        &pool,
        book.id,
        CreateAccountArgs {
            name: "Expenses",
            type_str: "expense",
            parent_id: None,
            commodity_id: None,
            description: None,
            placeholder: false,
            hidden: false,
        },
    )
    .await
    .unwrap();

    let accounts = list_accounts(&pool, book.id).await.unwrap();
    assert_eq!(accounts[0].commodity_id, commodity.id);
}

#[sqlx::test(migrations = "../storage/migrations")]
async fn create_with_invalid_type_errors(pool: SqlitePool) {
    let book = insert_book(&pool).await;
    insert_commodity(&pool, book.id).await;

    let err = cmd_create(
        &pool,
        book.id,
        CreateAccountArgs {
            name: "X",
            type_str: "not_a_type",
            parent_id: None,
            commodity_id: None,
            description: None,
            placeholder: false,
            hidden: false,
        },
    )
    .await;
    assert!(err.is_err());
    assert!(
        err.unwrap_err()
            .to_string()
            .contains("unknown account type")
    );
}

#[sqlx::test(migrations = "../storage/migrations")]
async fn create_with_missing_parent_errors(pool: SqlitePool) {
    let book = insert_book(&pool).await;
    insert_commodity(&pool, book.id).await;
    let fake_parent = uuid::Uuid::new_v4().to_string();

    let err = cmd_create(
        &pool,
        book.id,
        CreateAccountArgs {
            name: "Checking",
            type_str: "bank",
            parent_id: Some(fake_parent.as_str()),
            commodity_id: None,
            description: None,
            placeholder: false,
            hidden: false,
        },
    )
    .await;
    assert!(err.is_err());
}

// ── cmd_rename ────────────────────────────────────────────────────────────────

#[sqlx::test(migrations = "../storage/migrations")]
async fn rename_updates_name_and_full_name(pool: SqlitePool) {
    let book = insert_book(&pool).await;
    let commodity = insert_commodity(&pool, book.id).await;
    let repo = AccountRepository::new(pool.clone());
    let acct = make_account(
        book.id,
        commodity.id,
        "Checking",
        "Assets:Checking",
        AccountType::Bank,
    );
    repo.insert(&acct).await.unwrap();

    cmd_rename(&pool, book.id, &acct.id.to_string(), "Main Checking")
        .await
        .unwrap();

    let updated = get_account(&pool, &acct.id.to_string(), book.id)
        .await
        .unwrap();
    assert_eq!(updated.name, "Main Checking");
    assert_eq!(updated.full_name, "Main Checking");
}

#[sqlx::test(migrations = "../storage/migrations")]
async fn rename_by_full_name(pool: SqlitePool) {
    let book = insert_book(&pool).await;
    let commodity = insert_commodity(&pool, book.id).await;
    let repo = AccountRepository::new(pool.clone());
    let acct = make_account(
        book.id,
        commodity.id,
        "Checking",
        "Assets:Checking",
        AccountType::Bank,
    );
    repo.insert(&acct).await.unwrap();

    cmd_rename(&pool, book.id, "Assets:Checking", "Main Checking")
        .await
        .unwrap();

    let updated = get_account(&pool, &acct.id.to_string(), book.id)
        .await
        .unwrap();
    assert_eq!(updated.name, "Main Checking");
}

#[sqlx::test(migrations = "../storage/migrations")]
async fn rename_nonexistent_account_errors(pool: SqlitePool) {
    let book = insert_book(&pool).await;
    let err = cmd_rename(&pool, book.id, &uuid::Uuid::new_v4().to_string(), "X").await;
    assert!(err.is_err());
}

// ── cmd_delete ────────────────────────────────────────────────────────────────

#[sqlx::test(migrations = "../storage/migrations")]
async fn delete_soft_deletes_account(pool: SqlitePool) {
    let book = insert_book(&pool).await;
    let commodity = insert_commodity(&pool, book.id).await;
    let repo = AccountRepository::new(pool.clone());
    let acct = make_account(
        book.id,
        commodity.id,
        "OldAccount",
        "OldAccount",
        AccountType::Asset,
    );
    repo.insert(&acct).await.unwrap();

    cmd_delete(&pool, book.id, &acct.id.to_string())
        .await
        .unwrap();

    // soft-deleted account no longer appears in the active list
    let active = list_accounts(&pool, book.id).await.unwrap();
    assert!(active.is_empty());

    // but it still exists in the DB with deleted_at set
    let raw = repo.find_by_id(acct.id).await.unwrap().unwrap();
    assert!(raw.deleted_at.is_some());
}

#[sqlx::test(migrations = "../storage/migrations")]
async fn delete_by_full_name(pool: SqlitePool) {
    let book = insert_book(&pool).await;
    let commodity = insert_commodity(&pool, book.id).await;
    let repo = AccountRepository::new(pool.clone());
    let acct = make_account(
        book.id,
        commodity.id,
        "OldAccount",
        "OldAccount",
        AccountType::Asset,
    );
    repo.insert(&acct).await.unwrap();

    cmd_delete(&pool, book.id, "OldAccount").await.unwrap();

    let raw = repo.find_by_id(acct.id).await.unwrap().unwrap();
    assert!(raw.deleted_at.is_some());
}

#[sqlx::test(migrations = "../storage/migrations")]
async fn delete_nonexistent_account_errors(pool: SqlitePool) {
    let book = insert_book(&pool).await;
    let err = cmd_delete(&pool, book.id, &uuid::Uuid::new_v4().to_string()).await;
    assert!(err.is_err());
}

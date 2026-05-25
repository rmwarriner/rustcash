use chrono::Utc;
use rustcash_core::{
    account::{Account, AccountType},
    book::Book,
    commodity::Commodity,
    ids::{AccountId, BookId, CommodityId},
};
use rustcash_storage::{
    SqlitePool, StorageError,
    repositories::{
        accounts::AccountRepository, books::BookRepository, commodities::CommodityRepository,
    },
};

// ── fixtures ──────────────────────────────────────────────────────────────────

async fn insert_book(pool: &SqlitePool) -> Book {
    let commodity_id = CommodityId::new();
    let book = Book {
        id: BookId::new(),
        name: "Test Book".to_string(),
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

async fn insert_commodity(pool: &SqlitePool, book_id: BookId) -> Commodity {
    let c = Commodity {
        id: CommodityId::new(),
        book_id,
        namespace: "CURRENCY".to_string(),
        mnemonic: "USD".to_string(),
        name: "US Dollar".to_string(),
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

fn make_account(book_id: BookId, commodity_id: CommodityId) -> Account {
    Account {
        id: AccountId::new(),
        book_id,
        parent_id: None,
        name: "Assets".to_string(),
        full_name: "Assets".to_string(),
        account_type: AccountType::Asset,
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

// ── tests ─────────────────────────────────────────────────────────────────────

#[sqlx::test]
async fn insert_and_find_by_id(pool: SqlitePool) {
    let book = insert_book(&pool).await;
    let commodity = insert_commodity(&pool, book.id).await;
    let repo = AccountRepository::new(pool);
    let acct = make_account(book.id, commodity.id);
    repo.insert(&acct).await.unwrap();
    let found = repo.find_by_id(acct.id).await.unwrap().unwrap();
    assert_eq!(found.id, acct.id);
    assert_eq!(found.name, acct.name);
}

#[sqlx::test]
async fn find_by_id_returns_none_for_missing(pool: SqlitePool) {
    let repo = AccountRepository::new(pool);
    assert!(repo.find_by_id(AccountId::new()).await.unwrap().is_none());
}

#[sqlx::test]
async fn round_trip_preserves_all_fields(pool: SqlitePool) {
    let book = insert_book(&pool).await;
    let commodity = insert_commodity(&pool, book.id).await;
    let repo = AccountRepository::new(pool);
    let parent_id = AccountId::new();
    let now = Utc::now();
    let acct = Account {
        id: AccountId::new(),
        book_id: book.id,
        parent_id: Some(parent_id),
        name: "Checking".to_string(),
        full_name: "Assets:Checking".to_string(),
        account_type: AccountType::Bank,
        commodity_id: commodity.id,
        description: Some("Main checking account".to_string()),
        placeholder: true,
        hidden: true,
        sort_order: 42,
        created_at: now,
        modified_at: now,
        deleted_at: None,
    };
    // Insert parent shell so FK is satisfied
    let mut parent = make_account(book.id, commodity.id);
    parent.id = parent_id;
    repo.insert(&parent).await.unwrap();
    repo.insert(&acct).await.unwrap();
    let found = repo.find_by_id(acct.id).await.unwrap().unwrap();
    assert_eq!(found.id, acct.id);
    assert_eq!(found.book_id, acct.book_id);
    assert_eq!(found.parent_id, acct.parent_id);
    assert_eq!(found.name, acct.name);
    assert_eq!(found.full_name, acct.full_name);
    assert_eq!(found.account_type, acct.account_type);
    assert_eq!(found.commodity_id, acct.commodity_id);
    assert_eq!(found.description, acct.description);
    assert_eq!(found.placeholder, acct.placeholder);
    assert_eq!(found.hidden, acct.hidden);
    assert_eq!(found.sort_order, acct.sort_order);
    assert_eq!(found.created_at, acct.created_at);
    assert_eq!(found.modified_at, acct.modified_at);
    assert_eq!(found.deleted_at, acct.deleted_at);
}

#[sqlx::test]
async fn all_account_types_round_trip(pool: SqlitePool) {
    let book = insert_book(&pool).await;
    let commodity = insert_commodity(&pool, book.id).await;
    let repo = AccountRepository::new(pool);
    let types = [
        AccountType::Asset,
        AccountType::Cash,
        AccountType::Bank,
        AccountType::CreditCard,
        AccountType::Investment,
        AccountType::MutualFund,
        AccountType::Liability,
        AccountType::LongTermLiability,
        AccountType::Equity,
        AccountType::OpeningBalance,
        AccountType::RetainedEarnings,
        AccountType::Income,
        AccountType::Expense,
        AccountType::Receivable,
        AccountType::Payable,
    ];
    for account_type in types {
        let mut acct = make_account(book.id, commodity.id);
        acct.account_type = account_type;
        repo.insert(&acct).await.unwrap();
        let found = repo.find_by_id(acct.id).await.unwrap().unwrap();
        assert_eq!(found.account_type, account_type);
    }
}

#[sqlx::test]
async fn find_by_book_returns_active_accounts_ordered_by_full_name(pool: SqlitePool) {
    let book = insert_book(&pool).await;
    let commodity = insert_commodity(&pool, book.id).await;
    let repo = AccountRepository::new(pool);

    for (name, full_name) in [
        ("Expenses", "Expenses"),
        ("Assets", "Assets"),
        ("Income", "Income"),
    ] {
        let mut acct = make_account(book.id, commodity.id);
        acct.name = name.to_string();
        acct.full_name = full_name.to_string();
        repo.insert(&acct).await.unwrap();
    }

    let found = repo.find_by_book(book.id).await.unwrap();
    assert_eq!(found.len(), 3);
    assert_eq!(found[0].full_name, "Assets");
    assert_eq!(found[1].full_name, "Expenses");
    assert_eq!(found[2].full_name, "Income");
}

#[sqlx::test]
async fn find_by_book_excludes_soft_deleted(pool: SqlitePool) {
    let book = insert_book(&pool).await;
    let commodity = insert_commodity(&pool, book.id).await;
    let repo = AccountRepository::new(pool);
    let a1 = make_account(book.id, commodity.id);
    let a2 = make_account(book.id, commodity.id);
    repo.insert(&a1).await.unwrap();
    repo.insert(&a2).await.unwrap();
    repo.soft_delete(a1.id, Utc::now()).await.unwrap();
    let found = repo.find_by_book(book.id).await.unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].id, a2.id);
}

#[sqlx::test]
async fn find_by_book_excludes_other_books(pool: SqlitePool) {
    let book_a = insert_book(&pool).await;
    let book_b = insert_book(&pool).await;
    let c_a = insert_commodity(&pool, book_a.id).await;
    let c_b = insert_commodity(&pool, book_b.id).await;
    let repo = AccountRepository::new(pool);
    repo.insert(&make_account(book_a.id, c_a.id)).await.unwrap();
    repo.insert(&make_account(book_b.id, c_b.id)).await.unwrap();
    assert_eq!(repo.find_by_book(book_a.id).await.unwrap().len(), 1);
}

#[sqlx::test]
async fn find_children_returns_direct_children_only(pool: SqlitePool) {
    let book = insert_book(&pool).await;
    let commodity = insert_commodity(&pool, book.id).await;
    let repo = AccountRepository::new(pool);

    // root → child_a → grandchild
    let root = make_account(book.id, commodity.id);
    let mut child_a = make_account(book.id, commodity.id);
    let mut child_b = make_account(book.id, commodity.id);
    let mut grandchild = make_account(book.id, commodity.id);

    child_a.parent_id = Some(root.id);
    child_b.parent_id = Some(root.id);
    grandchild.parent_id = Some(child_a.id);

    for acct in [&root, &child_a, &child_b, &grandchild] {
        repo.insert(acct).await.unwrap();
    }

    let children = repo.find_children(root.id).await.unwrap();
    assert_eq!(children.len(), 2);
    assert!(children.iter().any(|a| a.id == child_a.id));
    assert!(children.iter().any(|a| a.id == child_b.id));
    // grandchild must not appear
    assert!(!children.iter().any(|a| a.id == grandchild.id));
}

#[sqlx::test]
async fn update_changes_fields(pool: SqlitePool) {
    let book = insert_book(&pool).await;
    let commodity = insert_commodity(&pool, book.id).await;
    let repo = AccountRepository::new(pool);
    let mut acct = make_account(book.id, commodity.id);
    repo.insert(&acct).await.unwrap();
    acct.name = "Cash".to_string();
    acct.full_name = "Assets:Cash".to_string();
    acct.account_type = AccountType::Cash;
    acct.placeholder = true;
    acct.sort_order = 5;
    acct.modified_at = Utc::now();
    repo.update(&acct).await.unwrap();
    let found = repo.find_by_id(acct.id).await.unwrap().unwrap();
    assert_eq!(found.name, "Cash");
    assert_eq!(found.full_name, "Assets:Cash");
    assert_eq!(found.account_type, AccountType::Cash);
    assert!(found.placeholder);
    assert_eq!(found.sort_order, 5);
}

#[sqlx::test]
async fn soft_delete_sets_deleted_at(pool: SqlitePool) {
    let book = insert_book(&pool).await;
    let commodity = insert_commodity(&pool, book.id).await;
    let repo = AccountRepository::new(pool);
    let acct = make_account(book.id, commodity.id);
    repo.insert(&acct).await.unwrap();
    let ts = Utc::now();
    repo.soft_delete(acct.id, ts).await.unwrap();
    let found = repo.find_by_id(acct.id).await.unwrap().unwrap();
    assert!(found.deleted_at.is_some());
}

#[sqlx::test]
async fn soft_delete_unknown_id_is_not_found(pool: SqlitePool) {
    let repo = AccountRepository::new(pool);
    let err = repo
        .soft_delete(AccountId::new(), Utc::now())
        .await
        .unwrap_err();
    assert!(matches!(err, StorageError::NotFound { .. }));
}

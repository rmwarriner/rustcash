use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;
use rustcash_core::{
    account::{Account, AccountType},
    book::Book,
    commodity::Commodity,
    ids::{AccountId, BookId, CommodityId, LotId, SplitId, TransactionId},
    transaction::{ReconcileState, Split, Transaction, TransactionStatus},
};
use rustcash_storage::{
    repositories::{
        accounts::AccountRepository,
        books::BookRepository,
        commodities::CommodityRepository,
        transactions::TransactionRepository,
    },
    SqlitePool, StorageError,
};

// ── fixtures ──────────────────────────────────────────────────────────────────

async fn insert_book(pool: &SqlitePool) -> Book {
    let book = Book {
        id:                   BookId::new(),
        name:                 "Test Book".to_string(),
        description:          None,
        default_commodity_id: CommodityId::new(),
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
        namespace:  "CURRENCY".to_string(),
        mnemonic:   "USD".to_string(),
        name:       "US Dollar".to_string(),
        fraction:   100,
        notes:      None,
        created_at: Utc::now(),
    };
    CommodityRepository::new(pool.clone()).insert(&c).await.unwrap();
    c
}

async fn insert_account(pool: &SqlitePool, book_id: BookId, commodity_id: CommodityId) -> Account {
    let acct = Account {
        id:           AccountId::new(),
        book_id,
        parent_id:    None,
        name:         "Test Account".to_string(),
        full_name:    "Test Account".to_string(),
        account_type: AccountType::Asset,
        commodity_id,
        description:  None,
        placeholder:  false,
        hidden:       false,
        sort_order:   0,
        created_at:   Utc::now(),
        modified_at:  Utc::now(),
        deleted_at:   None,
    };
    AccountRepository::new(pool.clone()).insert(&acct).await.unwrap();
    acct
}

fn make_split(account_id: AccountId, commodity_id: CommodityId, amount: Decimal) -> Split {
    Split {
        id:              SplitId::new(),
        account_id,
        amount,
        value:           amount,
        commodity_id,
        reconcile_state: ReconcileState::Unreconciled,
        reconcile_date:  None,
        memo:            None,
        tags:            Vec::new(),
        action:          None,
        lot_id:          None,
        created_at:      Utc::now(),
    }
}

/// Build a balanced two-split transaction (debit + credit).
fn make_transaction(
    book_id: BookId,
    debit_account: AccountId,
    credit_account: AccountId,
    commodity_id: CommodityId,
    amount: Decimal,
) -> Transaction {
    let splits = vec![
        make_split(debit_account,  commodity_id,  amount),
        make_split(credit_account, commodity_id, -amount),
    ];
    Transaction::new(
        TransactionId::new(),
        book_id,
        NaiveDate::from_ymd_opt(2024, 1, 15).unwrap(),
        "Groceries",
        splits,
    )
    .unwrap()
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[sqlx::test]
async fn insert_and_find_by_id(pool: SqlitePool) {
    let book = insert_book(&pool).await;
    let commodity = insert_commodity(&pool, book.id).await;
    let debit  = insert_account(&pool, book.id, commodity.id).await;
    let credit = insert_account(&pool, book.id, commodity.id).await;
    let repo = TransactionRepository::new(pool);

    let txn = make_transaction(book.id, debit.id, credit.id, commodity.id, Decimal::new(5000, 2));
    repo.insert(&txn).await.unwrap();

    let found = repo.find_by_id(txn.id).await.unwrap().unwrap();
    assert_eq!(found.id, txn.id);
    assert_eq!(found.description, "Groceries");
    assert_eq!(found.splits.len(), 2);
}

#[sqlx::test]
async fn find_by_id_returns_none_for_missing(pool: SqlitePool) {
    let repo = TransactionRepository::new(pool);
    assert!(repo.find_by_id(TransactionId::new()).await.unwrap().is_none());
}

#[sqlx::test]
async fn round_trip_preserves_all_fields(pool: SqlitePool) {
    let book = insert_book(&pool).await;
    let commodity = insert_commodity(&pool, book.id).await;
    let debit  = insert_account(&pool, book.id, commodity.id).await;
    let credit = insert_account(&pool, book.id, commodity.id).await;
    let repo = TransactionRepository::new(pool);

    let amount = Decimal::new(12345, 2);
    let lot = LotId::new();
    let mut txn = make_transaction(book.id, debit.id, credit.id, commodity.id, amount);
    txn.notes = Some("test note".to_string());
    txn.tags  = vec!["groceries".to_string(), "food".to_string()];
    // Enrich the debit split with all optional fields
    txn.splits[0].memo            = Some("split memo".to_string());
    txn.splits[0].tags            = vec!["debit-tag".to_string()];
    txn.splits[0].action          = Some("Buy".to_string());
    txn.splits[0].lot_id          = Some(lot);
    txn.splits[0].reconcile_state = ReconcileState::Cleared;
    txn.splits[0].reconcile_date  = Some(NaiveDate::from_ymd_opt(2024, 1, 16).unwrap());

    repo.insert(&txn).await.unwrap();
    let found = repo.find_by_id(txn.id).await.unwrap().unwrap();

    assert_eq!(found.id,          txn.id);
    assert_eq!(found.book_id,     txn.book_id);
    assert_eq!(found.date,        txn.date);
    assert_eq!(found.description, txn.description);
    assert_eq!(found.notes,       txn.notes);
    assert_eq!(found.tags,        txn.tags);
    assert_eq!(found.status,      txn.status);
    assert_eq!(found.splits.len(), 2);

    // Find the debit split by id
    let orig_debit = &txn.splits[0];
    let found_debit = found.splits.iter().find(|s| s.id == orig_debit.id).unwrap();
    assert_eq!(found_debit.account_id,      orig_debit.account_id);
    assert_eq!(found_debit.amount,          orig_debit.amount);
    assert_eq!(found_debit.value,           orig_debit.value);
    assert_eq!(found_debit.commodity_id,    orig_debit.commodity_id);
    assert_eq!(found_debit.reconcile_state, orig_debit.reconcile_state);
    assert_eq!(found_debit.reconcile_date,  orig_debit.reconcile_date);
    assert_eq!(found_debit.memo,            orig_debit.memo);
    assert_eq!(found_debit.tags,            orig_debit.tags);
    assert_eq!(found_debit.action,          orig_debit.action);
    assert_eq!(found_debit.lot_id,          orig_debit.lot_id);
}

#[sqlx::test]
async fn insert_is_atomic_rollback_on_fk_violation(pool: SqlitePool) {
    let book = insert_book(&pool).await;
    let commodity = insert_commodity(&pool, book.id).await;
    let debit  = insert_account(&pool, book.id, commodity.id).await;
    let credit = insert_account(&pool, book.id, commodity.id).await;
    let repo = TransactionRepository::new(pool);

    let mut txn = make_transaction(book.id, debit.id, credit.id, commodity.id, Decimal::new(100, 0));
    // Point one split at a non-existent account to force FK violation
    txn.splits[1].account_id = AccountId::new();

    let err = repo.insert(&txn).await.unwrap_err();
    assert!(matches!(err, StorageError::Database(_)), "expected DB error, got {err:?}");

    // The transaction row must not have been committed either
    assert!(repo.find_by_id(txn.id).await.unwrap().is_none());
}

#[sqlx::test]
async fn find_by_account_returns_full_transactions(pool: SqlitePool) {
    let book = insert_book(&pool).await;
    let commodity = insert_commodity(&pool, book.id).await;
    let checking  = insert_account(&pool, book.id, commodity.id).await;
    let groceries = insert_account(&pool, book.id, commodity.id).await;
    let repo = TransactionRepository::new(pool);

    let txn = make_transaction(book.id, checking.id, groceries.id, commodity.id, Decimal::new(5000, 2));
    repo.insert(&txn).await.unwrap();

    // Query by the groceries account — should still return both splits
    let found = repo.find_by_account(groceries.id, book.id).await.unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].id, txn.id);
    assert_eq!(found[0].splits.len(), 2);
}

#[sqlx::test]
async fn find_by_account_excludes_other_books(pool: SqlitePool) {
    let book_a = insert_book(&pool).await;
    let book_b = insert_book(&pool).await;
    let c_a = insert_commodity(&pool, book_a.id).await;
    let c_b = insert_commodity(&pool, book_b.id).await;
    let a1 = insert_account(&pool, book_a.id, c_a.id).await;
    let a2 = insert_account(&pool, book_a.id, c_a.id).await;
    let b1 = insert_account(&pool, book_b.id, c_b.id).await;
    let b2 = insert_account(&pool, book_b.id, c_b.id).await;
    let repo = TransactionRepository::new(pool);

    repo.insert(&make_transaction(book_a.id, a1.id, a2.id, c_a.id, Decimal::new(100, 0))).await.unwrap();
    repo.insert(&make_transaction(book_b.id, b1.id, b2.id, c_b.id, Decimal::new(200, 0))).await.unwrap();

    let results = repo.find_by_account(a1.id, book_a.id).await.unwrap();
    assert_eq!(results.len(), 1);
}

#[sqlx::test]
async fn update_status_posts_transaction(pool: SqlitePool) {
    let book = insert_book(&pool).await;
    let commodity = insert_commodity(&pool, book.id).await;
    let debit  = insert_account(&pool, book.id, commodity.id).await;
    let credit = insert_account(&pool, book.id, commodity.id).await;
    let repo = TransactionRepository::new(pool);

    let txn = make_transaction(book.id, debit.id, credit.id, commodity.id, Decimal::new(100, 0));
    repo.insert(&txn).await.unwrap();

    repo.update_status(txn.id, TransactionStatus::Posted, None, Utc::now())
        .await.unwrap();

    let found = repo.find_by_id(txn.id).await.unwrap().unwrap();
    assert_eq!(found.status, TransactionStatus::Posted);
    assert!(found.voiding_transaction_id.is_none());
}

#[sqlx::test]
async fn update_status_voids_transaction(pool: SqlitePool) {
    let book = insert_book(&pool).await;
    let commodity = insert_commodity(&pool, book.id).await;
    let debit  = insert_account(&pool, book.id, commodity.id).await;
    let credit = insert_account(&pool, book.id, commodity.id).await;
    let repo = TransactionRepository::new(pool);

    let txn      = make_transaction(book.id, debit.id, credit.id, commodity.id, Decimal::new(100, 0));
    let reversal = make_transaction(book.id, credit.id, debit.id, commodity.id, Decimal::new(100, 0));
    repo.insert(&txn).await.unwrap();
    repo.insert(&reversal).await.unwrap();

    repo.update_status(txn.id, TransactionStatus::Void, Some(reversal.id), Utc::now())
        .await.unwrap();

    let found = repo.find_by_id(txn.id).await.unwrap().unwrap();
    assert_eq!(found.status, TransactionStatus::Void);
    assert_eq!(found.voiding_transaction_id, Some(reversal.id));
}

#[sqlx::test]
async fn update_status_unknown_id_is_not_found(pool: SqlitePool) {
    let repo = TransactionRepository::new(pool);
    let err = repo
        .update_status(TransactionId::new(), TransactionStatus::Posted, None, Utc::now())
        .await
        .unwrap_err();
    assert!(matches!(err, StorageError::NotFound { .. }));
}

#[sqlx::test]
async fn update_split_reconcile_sets_state_and_date(pool: SqlitePool) {
    let book = insert_book(&pool).await;
    let commodity = insert_commodity(&pool, book.id).await;
    let debit  = insert_account(&pool, book.id, commodity.id).await;
    let credit = insert_account(&pool, book.id, commodity.id).await;
    let repo = TransactionRepository::new(pool);

    let txn = make_transaction(book.id, debit.id, credit.id, commodity.id, Decimal::new(100, 0));
    let split_id = txn.splits[0].id;
    repo.insert(&txn).await.unwrap();

    let reconcile_date = NaiveDate::from_ymd_opt(2024, 1, 20).unwrap();
    repo.update_split_reconcile(split_id, ReconcileState::Reconciled, Some(reconcile_date))
        .await.unwrap();

    let found = repo.find_by_id(txn.id).await.unwrap().unwrap();
    let split = found.splits.iter().find(|s| s.id == split_id).unwrap();
    assert_eq!(split.reconcile_state, ReconcileState::Reconciled);
    assert_eq!(split.reconcile_date,  Some(reconcile_date));
}

#[sqlx::test]
async fn update_split_reconcile_unknown_id_is_not_found(pool: SqlitePool) {
    let repo = TransactionRepository::new(pool);
    let err = repo
        .update_split_reconcile(SplitId::new(), ReconcileState::Cleared, None)
        .await
        .unwrap_err();
    assert!(matches!(err, StorageError::NotFound { .. }));
}

#[sqlx::test]
async fn find_by_account_returns_multiple_transactions_ordered_by_date(pool: SqlitePool) {
    let book = insert_book(&pool).await;
    let commodity = insert_commodity(&pool, book.id).await;
    let checking  = insert_account(&pool, book.id, commodity.id).await;
    let expenses  = insert_account(&pool, book.id, commodity.id).await;
    let repo = TransactionRepository::new(pool);

    // Insert in reverse date order to verify ORDER BY
    let mut txn_later = make_transaction(book.id, checking.id, expenses.id, commodity.id, Decimal::new(200, 0));
    txn_later.date = NaiveDate::from_ymd_opt(2024, 3, 1).unwrap();
    let mut txn_earlier = make_transaction(book.id, checking.id, expenses.id, commodity.id, Decimal::new(100, 0));
    txn_earlier.date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();

    repo.insert(&txn_later).await.unwrap();
    repo.insert(&txn_earlier).await.unwrap();

    let found = repo.find_by_account(checking.id, book.id).await.unwrap();
    assert_eq!(found.len(), 2);
    assert_eq!(found[0].date, txn_earlier.date);
    assert_eq!(found[1].date, txn_later.date);
}

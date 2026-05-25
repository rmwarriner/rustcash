mod helpers;

use chrono::NaiveDate;
use rust_decimal::Decimal;
use rustcash_core::{account::AccountType, ids::TransactionId, transaction::TransactionStatus};
use rustcash_engine::{EngineError, transaction::TransactionService};
use rustcash_storage::{SqlitePool, repositories::transactions::TransactionRepository};

use helpers::*;

fn jan(day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2024, 1, day).unwrap()
}

#[sqlx::test(migrations = "../storage/migrations")]
async fn enter_inserts_draft_transaction(pool: SqlitePool) {
    let book = insert_book(&pool).await;
    let commodity = insert_commodity(&pool, book.id).await;
    let checking =
        insert_account(&pool, book.id, commodity.id, "Checking", AccountType::Bank).await;
    let expenses = insert_account(
        &pool,
        book.id,
        commodity.id,
        "Expenses",
        AccountType::Expense,
    )
    .await;

    let txn = make_txn(
        book.id,
        checking.id,
        expenses.id,
        commodity.id,
        Decimal::new(100, 0),
        jan(5),
        "Coffee",
    );
    let svc = TransactionService::new(pool.clone());
    svc.enter(&txn).await.unwrap();

    let found = TransactionRepository::new(pool)
        .find_by_id(txn.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(found.status, TransactionStatus::Draft);
}

#[sqlx::test(migrations = "../storage/migrations")]
async fn post_transitions_draft_to_posted(pool: SqlitePool) {
    let book = insert_book(&pool).await;
    let commodity = insert_commodity(&pool, book.id).await;
    let checking =
        insert_account(&pool, book.id, commodity.id, "Checking", AccountType::Bank).await;
    let expenses = insert_account(
        &pool,
        book.id,
        commodity.id,
        "Expenses",
        AccountType::Expense,
    )
    .await;

    let txn = make_txn(
        book.id,
        checking.id,
        expenses.id,
        commodity.id,
        Decimal::new(100, 0),
        jan(5),
        "Coffee",
    );
    let svc = TransactionService::new(pool.clone());
    svc.enter(&txn).await.unwrap();
    svc.post(txn.id).await.unwrap();

    let found = TransactionRepository::new(pool)
        .find_by_id(txn.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(found.status, TransactionStatus::Posted);
}

#[sqlx::test(migrations = "../storage/migrations")]
async fn post_already_posted_returns_error(pool: SqlitePool) {
    let book = insert_book(&pool).await;
    let commodity = insert_commodity(&pool, book.id).await;
    let checking =
        insert_account(&pool, book.id, commodity.id, "Checking", AccountType::Bank).await;
    let expenses = insert_account(
        &pool,
        book.id,
        commodity.id,
        "Expenses",
        AccountType::Expense,
    )
    .await;

    let txn = make_txn(
        book.id,
        checking.id,
        expenses.id,
        commodity.id,
        Decimal::new(100, 0),
        jan(5),
        "Coffee",
    );
    let svc = TransactionService::new(pool.clone());
    svc.enter(&txn).await.unwrap();
    svc.post(txn.id).await.unwrap();

    let err = svc.post(txn.id).await.unwrap_err();
    assert!(matches!(err, EngineError::InvalidStatusTransition { .. }));
}

#[sqlx::test(migrations = "../storage/migrations")]
async fn post_void_transaction_returns_error(pool: SqlitePool) {
    let book = insert_book(&pool).await;
    let commodity = insert_commodity(&pool, book.id).await;
    let checking =
        insert_account(&pool, book.id, commodity.id, "Checking", AccountType::Bank).await;
    let expenses = insert_account(
        &pool,
        book.id,
        commodity.id,
        "Expenses",
        AccountType::Expense,
    )
    .await;

    let txn = make_txn(
        book.id,
        checking.id,
        expenses.id,
        commodity.id,
        Decimal::new(100, 0),
        jan(5),
        "Coffee",
    );
    let svc = TransactionService::new(pool.clone());
    svc.enter(&txn).await.unwrap();
    svc.post(txn.id).await.unwrap();
    svc.void(txn.id, None).await.unwrap();

    let err = svc.post(txn.id).await.unwrap_err();
    assert!(matches!(err, EngineError::InvalidStatusTransition { .. }));
}

#[sqlx::test(migrations = "../storage/migrations")]
async fn void_draft_transaction_returns_error(pool: SqlitePool) {
    let book = insert_book(&pool).await;
    let commodity = insert_commodity(&pool, book.id).await;
    let checking =
        insert_account(&pool, book.id, commodity.id, "Checking", AccountType::Bank).await;
    let expenses = insert_account(
        &pool,
        book.id,
        commodity.id,
        "Expenses",
        AccountType::Expense,
    )
    .await;

    let txn = make_txn(
        book.id,
        checking.id,
        expenses.id,
        commodity.id,
        Decimal::new(100, 0),
        jan(5),
        "Coffee",
    );
    let svc = TransactionService::new(pool.clone());
    svc.enter(&txn).await.unwrap();

    let err = svc.void(txn.id, None).await.unwrap_err();
    assert!(matches!(err, EngineError::InvalidStatusTransition { .. }));
}

#[sqlx::test(migrations = "../storage/migrations")]
async fn void_sets_status_to_void(pool: SqlitePool) {
    let book = insert_book(&pool).await;
    let commodity = insert_commodity(&pool, book.id).await;
    let checking =
        insert_account(&pool, book.id, commodity.id, "Checking", AccountType::Bank).await;
    let expenses = insert_account(
        &pool,
        book.id,
        commodity.id,
        "Expenses",
        AccountType::Expense,
    )
    .await;

    let txn = make_txn(
        book.id,
        checking.id,
        expenses.id,
        commodity.id,
        Decimal::new(100, 0),
        jan(5),
        "Coffee",
    );
    let svc = TransactionService::new(pool.clone());
    svc.enter(&txn).await.unwrap();
    svc.post(txn.id).await.unwrap();
    svc.void(txn.id, None).await.unwrap();

    let found = TransactionRepository::new(pool)
        .find_by_id(txn.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(found.status, TransactionStatus::Void);
    assert!(found.voiding_transaction_id.is_none());
}

#[sqlx::test(migrations = "../storage/migrations")]
async fn void_with_replacement_records_link(pool: SqlitePool) {
    let book = insert_book(&pool).await;
    let commodity = insert_commodity(&pool, book.id).await;
    let checking =
        insert_account(&pool, book.id, commodity.id, "Checking", AccountType::Bank).await;
    let expenses = insert_account(
        &pool,
        book.id,
        commodity.id,
        "Expenses",
        AccountType::Expense,
    )
    .await;

    let svc = TransactionService::new(pool.clone());

    // Original transaction
    let orig = make_txn(
        book.id,
        checking.id,
        expenses.id,
        commodity.id,
        Decimal::new(100, 0),
        jan(5),
        "Coffee",
    );
    svc.enter(&orig).await.unwrap();
    svc.post(orig.id).await.unwrap();

    // Replacement (correcting) transaction
    let replacement = make_txn(
        book.id,
        checking.id,
        expenses.id,
        commodity.id,
        Decimal::new(95, 0),
        jan(5),
        "Coffee (corrected)",
    );
    svc.enter(&replacement).await.unwrap();
    svc.post(replacement.id).await.unwrap();

    // Void original, linking to replacement
    svc.void(orig.id, Some(replacement.id)).await.unwrap();

    let found = TransactionRepository::new(pool)
        .find_by_id(orig.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(found.status, TransactionStatus::Void);
    assert_eq!(found.voiding_transaction_id, Some(replacement.id));
}

#[sqlx::test(migrations = "../storage/migrations")]
async fn post_unknown_transaction_is_not_found(pool: SqlitePool) {
    let svc = TransactionService::new(pool);
    let err = svc.post(TransactionId::new()).await.unwrap_err();
    assert!(matches!(err, EngineError::TransactionNotFound { .. }));
}

#[sqlx::test(migrations = "../storage/migrations")]
async fn void_unknown_transaction_is_not_found(pool: SqlitePool) {
    let svc = TransactionService::new(pool);
    let err = svc.void(TransactionId::new(), None).await.unwrap_err();
    assert!(matches!(err, EngineError::TransactionNotFound { .. }));
}

#[sqlx::test(migrations = "../storage/migrations")]
async fn balance_after_void_returns_to_zero(pool: SqlitePool) {
    use rustcash_engine::balance::BalanceService;

    let book = insert_book(&pool).await;
    let commodity = insert_commodity(&pool, book.id).await;
    let checking =
        insert_account(&pool, book.id, commodity.id, "Checking", AccountType::Bank).await;
    let expenses = insert_account(
        &pool,
        book.id,
        commodity.id,
        "Expenses",
        AccountType::Expense,
    )
    .await;

    let txn = make_txn(
        book.id,
        checking.id,
        expenses.id,
        commodity.id,
        Decimal::new(100, 0),
        jan(5),
        "Coffee",
    );
    let svc = TransactionService::new(pool.clone());
    svc.enter(&txn).await.unwrap();
    svc.post(txn.id).await.unwrap();

    let bal_before = BalanceService::new(pool.clone())
        .account_balance(checking.id, book.id, jan(31))
        .await
        .unwrap();
    assert_eq!(bal_before.balance, Decimal::new(100, 0));

    svc.void(txn.id, None).await.unwrap();

    let bal_after = BalanceService::new(pool)
        .account_balance(checking.id, book.id, jan(31))
        .await
        .unwrap();
    assert_eq!(bal_after.balance, Decimal::ZERO);
}

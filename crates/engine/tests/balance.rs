mod helpers;

use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;
use rustcash_core::{
    account::AccountType,
    transaction::{ReconcileState, TransactionStatus},
};
use rustcash_engine::balance::BalanceService;
use rustcash_storage::{SqlitePool, repositories::transactions::TransactionRepository};

use helpers::*;

fn jan(day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2024, 1, day).unwrap()
}

#[sqlx::test(migrations = "../storage/migrations")]
async fn empty_account_has_zero_balance(pool: SqlitePool) {
    let book = insert_book(&pool).await;
    let commodity = insert_commodity(&pool, book.id).await;
    let account =
        insert_account_full(&pool, book.id, commodity.id, "Checking", AccountType::Bank).await;

    let svc = BalanceService::new(pool);
    let bal = svc
        .account_balance(account.id, book.id, jan(31))
        .await
        .unwrap();

    assert_eq!(bal.balance, Decimal::ZERO);
    assert_eq!(bal.cleared_balance, Decimal::ZERO);
    assert_eq!(bal.reconciled_balance, Decimal::ZERO);
}

#[sqlx::test(migrations = "../storage/migrations")]
async fn draft_transactions_excluded_from_balance(pool: SqlitePool) {
    let book = insert_book(&pool).await;
    let commodity = insert_commodity(&pool, book.id).await;
    let checking =
        insert_account_full(&pool, book.id, commodity.id, "Checking", AccountType::Bank).await;
    let expenses = insert_account_full(
        &pool,
        book.id,
        commodity.id,
        "Expenses",
        AccountType::Expense,
    )
    .await;

    // Insert a Draft (default status from Transaction::new)
    let txn = make_txn(
        book.id,
        checking.id,
        expenses.id,
        commodity.id,
        Decimal::new(100, 0),
        jan(5),
        "Coffee",
    );
    insert_txn(&pool, &txn).await;

    let svc = BalanceService::new(pool);
    let bal = svc
        .account_balance(checking.id, book.id, jan(31))
        .await
        .unwrap();
    assert_eq!(bal.balance, Decimal::ZERO);
}

#[sqlx::test(migrations = "../storage/migrations")]
async fn posted_transactions_included_in_balance(pool: SqlitePool) {
    let book = insert_book(&pool).await;
    let commodity = insert_commodity(&pool, book.id).await;
    let checking =
        insert_account_full(&pool, book.id, commodity.id, "Checking", AccountType::Bank).await;
    let expenses = insert_account_full(
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
    insert_txn(&pool, &txn).await;

    // Post it via storage directly so we can isolate balance logic
    TransactionRepository::new(pool.clone())
        .update_status(txn.id, TransactionStatus::Posted, None, Utc::now())
        .await
        .unwrap();

    let svc = BalanceService::new(pool);
    let bal = svc
        .account_balance(checking.id, book.id, jan(31))
        .await
        .unwrap();
    assert_eq!(bal.balance, Decimal::new(100, 0));
}

#[sqlx::test(migrations = "../storage/migrations")]
async fn void_transactions_excluded_from_balance(pool: SqlitePool) {
    let book = insert_book(&pool).await;
    let commodity = insert_commodity(&pool, book.id).await;
    let checking =
        insert_account_full(&pool, book.id, commodity.id, "Checking", AccountType::Bank).await;
    let expenses = insert_account_full(
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
    insert_txn(&pool, &txn).await;

    let repo = TransactionRepository::new(pool.clone());
    repo.update_status(txn.id, TransactionStatus::Posted, None, Utc::now())
        .await
        .unwrap();
    repo.update_status(txn.id, TransactionStatus::Void, None, Utc::now())
        .await
        .unwrap();

    let svc = BalanceService::new(pool);
    let bal = svc
        .account_balance(checking.id, book.id, jan(31))
        .await
        .unwrap();
    assert_eq!(bal.balance, Decimal::ZERO);
}

#[sqlx::test(migrations = "../storage/migrations")]
async fn balance_respects_as_of_date(pool: SqlitePool) {
    let book = insert_book(&pool).await;
    let commodity = insert_commodity(&pool, book.id).await;
    let checking =
        insert_account_full(&pool, book.id, commodity.id, "Checking", AccountType::Bank).await;
    let expenses = insert_account_full(
        &pool,
        book.id,
        commodity.id,
        "Expenses",
        AccountType::Expense,
    )
    .await;

    let repo = TransactionRepository::new(pool.clone());

    let t1 = make_txn(
        book.id,
        checking.id,
        expenses.id,
        commodity.id,
        Decimal::new(100, 0),
        jan(5),
        "Jan 5",
    );
    let t2 = make_txn(
        book.id,
        checking.id,
        expenses.id,
        commodity.id,
        Decimal::new(200, 0),
        jan(20),
        "Jan 20",
    );
    insert_txn(&pool, &t1).await;
    insert_txn(&pool, &t2).await;
    repo.update_status(t1.id, TransactionStatus::Posted, None, Utc::now())
        .await
        .unwrap();
    repo.update_status(t2.id, TransactionStatus::Posted, None, Utc::now())
        .await
        .unwrap();

    let svc = BalanceService::new(pool);
    let bal = svc
        .account_balance(checking.id, book.id, jan(10))
        .await
        .unwrap();
    assert_eq!(bal.balance, Decimal::new(100, 0));
}

#[sqlx::test(migrations = "../storage/migrations")]
async fn multiple_posted_transactions_sum_correctly(pool: SqlitePool) {
    let book = insert_book(&pool).await;
    let commodity = insert_commodity(&pool, book.id).await;
    let checking =
        insert_account_full(&pool, book.id, commodity.id, "Checking", AccountType::Bank).await;
    let expenses = insert_account_full(
        &pool,
        book.id,
        commodity.id,
        "Expenses",
        AccountType::Expense,
    )
    .await;

    let repo = TransactionRepository::new(pool.clone());
    for (amount, day) in [(100, 1u32), (250, 5), (75, 10)] {
        let t = make_txn(
            book.id,
            checking.id,
            expenses.id,
            commodity.id,
            Decimal::new(amount, 0),
            jan(day),
            "x",
        );
        insert_txn(&pool, &t).await;
        repo.update_status(t.id, TransactionStatus::Posted, None, Utc::now())
            .await
            .unwrap();
    }

    let svc = BalanceService::new(pool);
    let bal = svc
        .account_balance(checking.id, book.id, jan(31))
        .await
        .unwrap();
    assert_eq!(bal.balance, Decimal::new(425, 0));
}

#[sqlx::test(migrations = "../storage/migrations")]
async fn cleared_balance_counts_cleared_and_reconciled_splits(pool: SqlitePool) {
    let book = insert_book(&pool).await;
    let commodity = insert_commodity(&pool, book.id).await;
    let checking =
        insert_account_full(&pool, book.id, commodity.id, "Checking", AccountType::Bank).await;
    let expenses = insert_account_full(
        &pool,
        book.id,
        commodity.id,
        "Expenses",
        AccountType::Expense,
    )
    .await;

    let repo = TransactionRepository::new(pool.clone());

    // t1: unreconciled
    let t1 = make_txn(
        book.id,
        checking.id,
        expenses.id,
        commodity.id,
        Decimal::new(100, 0),
        jan(1),
        "a",
    );
    insert_txn(&pool, &t1).await;
    repo.update_status(t1.id, TransactionStatus::Posted, None, Utc::now())
        .await
        .unwrap();

    // t2: cleared
    let t2 = make_txn(
        book.id,
        checking.id,
        expenses.id,
        commodity.id,
        Decimal::new(200, 0),
        jan(2),
        "b",
    );
    insert_txn(&pool, &t2).await;
    repo.update_status(t2.id, TransactionStatus::Posted, None, Utc::now())
        .await
        .unwrap();
    let checking_split_id = t2
        .splits
        .iter()
        .find(|s| s.account_id == checking.id)
        .unwrap()
        .id;
    repo.update_split_reconcile(checking_split_id, ReconcileState::Cleared, None)
        .await
        .unwrap();

    // t3: reconciled
    let t3 = make_txn(
        book.id,
        checking.id,
        expenses.id,
        commodity.id,
        Decimal::new(300, 0),
        jan(3),
        "c",
    );
    insert_txn(&pool, &t3).await;
    repo.update_status(t3.id, TransactionStatus::Posted, None, Utc::now())
        .await
        .unwrap();
    let checking_split_id = t3
        .splits
        .iter()
        .find(|s| s.account_id == checking.id)
        .unwrap()
        .id;
    repo.update_split_reconcile(checking_split_id, ReconcileState::Reconciled, None)
        .await
        .unwrap();

    let svc = BalanceService::new(pool);
    let bal = svc
        .account_balance(checking.id, book.id, jan(31))
        .await
        .unwrap();
    assert_eq!(bal.balance, Decimal::new(600, 0));
    assert_eq!(bal.cleared_balance, Decimal::new(500, 0)); // cleared + reconciled
    assert_eq!(bal.reconciled_balance, Decimal::new(300, 0)); // reconciled only
}

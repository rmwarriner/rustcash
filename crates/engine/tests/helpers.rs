//! Shared test fixtures for engine integration tests.
#![allow(dead_code)]

use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;
use rustcash_core::{
    account::{Account, AccountType},
    book::Book,
    commodity::Commodity,
    ids::{AccountId, BookId, CommodityId, SplitId, TransactionId},
    transaction::{Split, Transaction},
};
use rustcash_storage::{
    SqlitePool,
    repositories::{
        accounts::AccountRepository, books::BookRepository, commodities::CommodityRepository,
        transactions::TransactionRepository,
    },
};

pub async fn insert_book(pool: &SqlitePool) -> Book {
    let book = Book {
        id: BookId::new(),
        name: "Test Book".to_string(),
        description: None,
        default_commodity_id: CommodityId::new(),
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

pub async fn insert_commodity(pool: &SqlitePool, book_id: BookId) -> Commodity {
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

pub async fn insert_account(
    pool: &SqlitePool,
    book_id: BookId,
    commodity_id: CommodityId,
    name: &str,
    account_type: AccountType,
) -> Account {
    let acct = Account {
        id: AccountId::new(),
        book_id,
        parent_id: None,
        name: name.to_string(),
        full_name: name.to_string(),
        account_type,
        commodity_id,
        description: None,
        placeholder: false,
        hidden: false,
        sort_order: 0,
        created_at: Utc::now(),
        modified_at: Utc::now(),
        deleted_at: None,
    };
    AccountRepository::new(pool.clone())
        .insert(&acct)
        .await
        .unwrap();
    acct
}

pub fn make_split(account_id: AccountId, commodity_id: CommodityId, amount: Decimal) -> Split {
    Split {
        id: SplitId::new(),
        account_id,
        amount,
        value: amount,
        commodity_id,
        reconcile_state: rustcash_core::transaction::ReconcileState::Unreconciled,
        reconcile_date: None,
        memo: None,
        tags: Vec::new(),
        action: None,
        lot_id: None,
        created_at: Utc::now(),
    }
}

/// Build a balanced two-split transaction.
pub fn make_txn(
    book_id: BookId,
    debit: AccountId,
    credit: AccountId,
    commodity_id: CommodityId,
    amount: Decimal,
    date: NaiveDate,
    description: &str,
) -> Transaction {
    Transaction::new(
        TransactionId::new(),
        book_id,
        date,
        description,
        vec![
            make_split(debit, commodity_id, amount),
            make_split(credit, commodity_id, -amount),
        ],
    )
    .unwrap()
}

pub async fn insert_txn(pool: &SqlitePool, txn: &Transaction) {
    TransactionRepository::new(pool.clone())
        .insert(txn)
        .await
        .unwrap();
}

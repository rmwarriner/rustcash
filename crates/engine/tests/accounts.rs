mod helpers;

use chrono::Utc;
use rustcash_core::{
    account::{Account, AccountType},
    ids::AccountId,
};
use rustcash_engine::{EngineError, account::AccountService};
use rustcash_storage::{SqlitePool, repositories::accounts::AccountRepository};

use helpers::*;

#[sqlx::test(migrations = "../storage/migrations")]
async fn create_inserts_account(pool: SqlitePool) {
    let book = insert_book(&pool).await;
    let commodity = insert_commodity(&pool, book.id).await;

    let acct = Account {
        id: AccountId::new(),
        book_id: book.id,
        parent_id: None,
        name: "Assets".to_string(),
        full_name: "Assets".to_string(),
        account_type: AccountType::Asset,
        commodity_id: commodity.id,
        description: None,
        placeholder: false,
        hidden: false,
        sort_order: 0,
        created_at: Utc::now(),
        modified_at: Utc::now(),
        deleted_at: None,
    };
    AccountService::new(pool.clone())
        .create(&acct)
        .await
        .unwrap();

    let found = AccountRepository::new(pool)
        .find_by_id(acct.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(found.id, acct.id);
    assert_eq!(found.name, "Assets");
}

#[sqlx::test(migrations = "../storage/migrations")]
async fn create_with_invalid_parent_returns_error(pool: SqlitePool) {
    let book = insert_book(&pool).await;
    let commodity = insert_commodity(&pool, book.id).await;

    let acct = Account {
        id: AccountId::new(),
        book_id: book.id,
        parent_id: Some(AccountId::new()), // nonexistent
        name: "Checking".to_string(),
        full_name: "Assets:Checking".to_string(),
        account_type: AccountType::Bank,
        commodity_id: commodity.id,
        description: None,
        placeholder: false,
        hidden: false,
        sort_order: 0,
        created_at: Utc::now(),
        modified_at: Utc::now(),
        deleted_at: None,
    };
    let err = AccountService::new(pool).create(&acct).await.unwrap_err();
    assert!(matches!(err, EngineError::AccountNotFound { .. }));
}

#[sqlx::test(migrations = "../storage/migrations")]
async fn rename_updates_name_and_full_name(pool: SqlitePool) {
    let book = insert_book(&pool).await;
    let commodity = insert_commodity(&pool, book.id).await;
    let acct = insert_account(&pool, book.id, commodity.id, "Assets", AccountType::Asset).await;

    AccountService::new(pool.clone())
        .rename(
            acct.id,
            "Current Assets".to_string(),
            "Current Assets".to_string(),
        )
        .await
        .unwrap();

    let found = AccountRepository::new(pool)
        .find_by_id(acct.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(found.name, "Current Assets");
    assert_eq!(found.full_name, "Current Assets");
}

#[sqlx::test(migrations = "../storage/migrations")]
async fn rename_unknown_account_returns_error(pool: SqlitePool) {
    let err = AccountService::new(pool)
        .rename(AccountId::new(), "X".to_string(), "X".to_string())
        .await
        .unwrap_err();
    assert!(matches!(err, EngineError::AccountNotFound { .. }));
}

#[sqlx::test(migrations = "../storage/migrations")]
async fn rename_cascades_full_name_to_children(pool: SqlitePool) {
    let book = insert_book(&pool).await;
    let commodity = insert_commodity(&pool, book.id).await;
    let repo = AccountRepository::new(pool.clone());

    // root → child → grandchild
    let root = insert_account(&pool, book.id, commodity.id, "Assets", AccountType::Asset).await;

    let mut child = insert_account(&pool, book.id, commodity.id, "Cash", AccountType::Cash).await;
    child.parent_id = Some(root.id);
    child.full_name = "Assets:Cash".to_string();
    repo.update(&child).await.unwrap();

    let mut grandchild =
        insert_account(&pool, book.id, commodity.id, "Petty", AccountType::Cash).await;
    grandchild.parent_id = Some(child.id);
    grandchild.full_name = "Assets:Cash:Petty".to_string();
    repo.update(&grandchild).await.unwrap();

    AccountService::new(pool.clone())
        .rename(
            root.id,
            "Current Assets".to_string(),
            "Current Assets".to_string(),
        )
        .await
        .unwrap();

    let found_child = repo.find_by_id(child.id).await.unwrap().unwrap();
    let found_grand = repo.find_by_id(grandchild.id).await.unwrap().unwrap();
    assert_eq!(found_child.full_name, "Current Assets:Cash");
    assert_eq!(found_grand.full_name, "Current Assets:Cash:Petty");
}

#[sqlx::test(migrations = "../storage/migrations")]
async fn soft_delete_removes_from_active_accounts(pool: SqlitePool) {
    let book = insert_book(&pool).await;
    let commodity = insert_commodity(&pool, book.id).await;
    let acct = insert_account(&pool, book.id, commodity.id, "Checking", AccountType::Bank).await;

    AccountService::new(pool.clone())
        .soft_delete(acct.id)
        .await
        .unwrap();

    let active = AccountRepository::new(pool)
        .find_by_book(book.id)
        .await
        .unwrap();
    assert!(active.is_empty());
}

#[sqlx::test(migrations = "../storage/migrations")]
async fn soft_delete_unknown_account_returns_error(pool: SqlitePool) {
    let err = AccountService::new(pool)
        .soft_delete(AccountId::new())
        .await
        .unwrap_err();
    assert!(matches!(err, EngineError::Storage(_)));
}

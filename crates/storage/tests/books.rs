use chrono::{NaiveDate, Utc};
use rustcash_core::{
    book::Book,
    ids::{BookId, CommodityId},
};
use rustcash_storage::{SqlitePool, StorageError, repositories::books::BookRepository};

fn make_book() -> Book {
    Book {
        id: BookId::new(),
        name: "Test Book".to_string(),
        description: None,
        default_commodity_id: CommodityId::new(),
        period_close_date: None,
        owner_id: None,
        created_at: Utc::now(),
        modified_at: Utc::now(),
        deleted_at: None,
    }
}

#[sqlx::test]
async fn insert_and_find_by_id(pool: SqlitePool) {
    let repo = BookRepository::new(pool);
    let book = make_book();
    repo.insert(&book).await.unwrap();
    let found = repo.find_by_id(book.id).await.unwrap().unwrap();
    assert_eq!(found.id, book.id);
    assert_eq!(found.name, book.name);
}

#[sqlx::test]
async fn find_by_id_returns_none_for_missing(pool: SqlitePool) {
    let repo = BookRepository::new(pool);
    assert!(repo.find_by_id(BookId::new()).await.unwrap().is_none());
}

#[sqlx::test]
async fn round_trip_preserves_all_fields(pool: SqlitePool) {
    let repo = BookRepository::new(pool);
    let now = Utc::now();
    let book = Book {
        id: BookId::new(),
        name: "My Finances".to_string(),
        description: Some("Personal accounts".to_string()),
        default_commodity_id: CommodityId::new(),
        period_close_date: Some(NaiveDate::from_ymd_opt(2025, 6, 30).unwrap()),
        owner_id: None,
        created_at: now,
        modified_at: now,
        deleted_at: None,
    };
    repo.insert(&book).await.unwrap();
    let found = repo.find_by_id(book.id).await.unwrap().unwrap();
    assert_eq!(found.id, book.id);
    assert_eq!(found.name, book.name);
    assert_eq!(found.description, book.description);
    assert_eq!(found.default_commodity_id, book.default_commodity_id);
    assert_eq!(found.period_close_date, book.period_close_date);
    assert_eq!(found.owner_id, book.owner_id);
    assert_eq!(found.created_at, book.created_at);
    assert_eq!(found.modified_at, book.modified_at);
    assert_eq!(found.deleted_at, book.deleted_at);
}

#[sqlx::test]
async fn find_all_excludes_soft_deleted(pool: SqlitePool) {
    let repo = BookRepository::new(pool);
    let b1 = make_book();
    let b2 = make_book();
    repo.insert(&b1).await.unwrap();
    repo.insert(&b2).await.unwrap();
    repo.soft_delete(b1.id, Utc::now()).await.unwrap();
    let all = repo.find_all().await.unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].id, b2.id);
}

#[sqlx::test]
async fn update_changes_fields(pool: SqlitePool) {
    let repo = BookRepository::new(pool);
    let mut book = make_book();
    repo.insert(&book).await.unwrap();
    book.name = "Updated Name".to_string();
    book.description = Some("Now with description".to_string());
    book.modified_at = Utc::now();
    repo.update(&book).await.unwrap();
    let found = repo.find_by_id(book.id).await.unwrap().unwrap();
    assert_eq!(found.name, "Updated Name");
    assert_eq!(found.description, Some("Now with description".to_string()));
}

#[sqlx::test]
async fn soft_delete_unknown_id_is_not_found(pool: SqlitePool) {
    let repo = BookRepository::new(pool);
    let err = repo
        .soft_delete(BookId::new(), Utc::now())
        .await
        .unwrap_err();
    assert!(matches!(err, StorageError::NotFound { .. }));
}

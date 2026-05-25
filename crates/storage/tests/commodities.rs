use chrono::Utc;
use rustcash_core::{
    book::Book,
    commodity::Commodity,
    ids::{BookId, CommodityId},
};
use rustcash_storage::{
    repositories::{books::BookRepository, commodities::CommodityRepository},
    SqlitePool, StorageError,
};

// ── fixtures ─────────────────────────────────────────────────────────────────

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

fn make_commodity(book_id: BookId) -> Commodity {
    Commodity {
        id:         CommodityId::new(),
        book_id,
        namespace:  "CURRENCY".to_string(),
        mnemonic:   "USD".to_string(),
        name:       "US Dollar".to_string(),
        fraction:   100,
        notes:      None,
        created_at: Utc::now(),
    }
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[sqlx::test]
async fn insert_and_find_by_id(pool: SqlitePool) {
    let book = insert_book(&pool).await;
    let repo = CommodityRepository::new(pool);
    let c = make_commodity(book.id);
    repo.insert(&c).await.unwrap();
    let found = repo.find_by_id(c.id).await.unwrap().unwrap();
    assert_eq!(found.id, c.id);
    assert_eq!(found.mnemonic, c.mnemonic);
}

#[sqlx::test]
async fn find_by_id_returns_none_for_missing(pool: SqlitePool) {
    let repo = CommodityRepository::new(pool);
    assert!(repo.find_by_id(CommodityId::new()).await.unwrap().is_none());
}

#[sqlx::test]
async fn round_trip_preserves_all_fields(pool: SqlitePool) {
    let book = insert_book(&pool).await;
    let repo = CommodityRepository::new(pool);
    let c = Commodity {
        id:         CommodityId::new(),
        book_id:    book.id,
        namespace:  "NYSE".to_string(),
        mnemonic:   "AAPL".to_string(),
        name:       "Apple Inc.".to_string(),
        fraction:   1000,
        notes:      Some("Tech stock".to_string()),
        created_at: Utc::now(),
    };
    repo.insert(&c).await.unwrap();
    let found = repo.find_by_id(c.id).await.unwrap().unwrap();
    assert_eq!(found.id,         c.id);
    assert_eq!(found.book_id,    c.book_id);
    assert_eq!(found.namespace,  c.namespace);
    assert_eq!(found.mnemonic,   c.mnemonic);
    assert_eq!(found.name,       c.name);
    assert_eq!(found.fraction,   c.fraction);
    assert_eq!(found.notes,      c.notes);
    assert_eq!(found.created_at, c.created_at);
}

#[sqlx::test]
async fn find_by_book_returns_only_that_books_commodities(pool: SqlitePool) {
    let book_a = insert_book(&pool).await;
    let book_b = insert_book(&pool).await;
    let repo = CommodityRepository::new(pool);

    let mut usd = make_commodity(book_a.id);
    usd.mnemonic = "USD".to_string();
    let mut eur = make_commodity(book_a.id);
    eur.mnemonic = "EUR".to_string();
    let mut gbp = make_commodity(book_b.id);
    gbp.mnemonic = "GBP".to_string();

    repo.insert(&usd).await.unwrap();
    repo.insert(&eur).await.unwrap();
    repo.insert(&gbp).await.unwrap();

    let found = repo.find_by_book(book_a.id).await.unwrap();
    assert_eq!(found.len(), 2);
    assert!(found.iter().any(|c| c.mnemonic == "EUR"));
    assert!(found.iter().any(|c| c.mnemonic == "USD"));
}

#[sqlx::test]
async fn find_by_book_returns_ordered_by_namespace_mnemonic(pool: SqlitePool) {
    let book = insert_book(&pool).await;
    let repo = CommodityRepository::new(pool);

    for mnemonic in ["USD", "EUR", "GBP"] {
        let mut c = make_commodity(book.id);
        c.id = CommodityId::new();
        c.mnemonic = mnemonic.to_string();
        repo.insert(&c).await.unwrap();
    }

    let found = repo.find_by_book(book.id).await.unwrap();
    let mnemonics: Vec<&str> = found.iter().map(|c| c.mnemonic.as_str()).collect();
    assert_eq!(mnemonics, ["EUR", "GBP", "USD"]);
}

#[sqlx::test]
async fn find_by_mnemonic_returns_match(pool: SqlitePool) {
    let book = insert_book(&pool).await;
    let repo = CommodityRepository::new(pool);
    let c = make_commodity(book.id);
    repo.insert(&c).await.unwrap();
    let found = repo
        .find_by_mnemonic(book.id, "CURRENCY", "USD")
        .await
        .unwrap();
    assert_eq!(found.unwrap().id, c.id);
}

#[sqlx::test]
async fn find_by_mnemonic_returns_none_for_wrong_book(pool: SqlitePool) {
    let book_a = insert_book(&pool).await;
    let book_b = insert_book(&pool).await;
    let repo = CommodityRepository::new(pool);
    repo.insert(&make_commodity(book_a.id)).await.unwrap();
    assert!(repo
        .find_by_mnemonic(book_b.id, "CURRENCY", "USD")
        .await
        .unwrap()
        .is_none());
}

#[sqlx::test]
async fn insert_duplicate_mnemonic_is_constraint_error(pool: SqlitePool) {
    let book = insert_book(&pool).await;
    let repo = CommodityRepository::new(pool);
    repo.insert(&make_commodity(book.id)).await.unwrap();
    let err = repo.insert(&make_commodity(book.id)).await.unwrap_err();
    assert!(matches!(err, StorageError::Constraint(_)));
}

#[sqlx::test]
async fn update_changes_mutable_fields(pool: SqlitePool) {
    let book = insert_book(&pool).await;
    let repo = CommodityRepository::new(pool);
    let mut c = make_commodity(book.id);
    repo.insert(&c).await.unwrap();
    c.name = "United States Dollar".to_string();
    c.fraction = 1000;
    c.notes = Some("Updated".to_string());
    repo.update(&c).await.unwrap();
    let found = repo.find_by_id(c.id).await.unwrap().unwrap();
    assert_eq!(found.name, "United States Dollar");
    assert_eq!(found.fraction, 1000);
    assert_eq!(found.notes, Some("Updated".to_string()));
    // immutable fields unchanged
    assert_eq!(found.namespace, "CURRENCY");
    assert_eq!(found.mnemonic, "USD");
}

#[sqlx::test]
async fn update_unknown_id_is_not_found(pool: SqlitePool) {
    let book = insert_book(&pool).await;
    let repo = CommodityRepository::new(pool);
    let c = make_commodity(book.id);
    let err = repo.update(&c).await.unwrap_err();
    assert!(matches!(err, StorageError::NotFound { .. }));
}

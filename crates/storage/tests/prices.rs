use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;
use rustcash_core::{
    book::Book,
    commodity::{Commodity, Price, PriceSource},
    ids::{BookId, CommodityId, PriceId},
};
use rustcash_storage::{
    SqlitePool, StorageError,
    repositories::{
        books::BookRepository, commodities::CommodityRepository, prices::PriceRepository,
    },
};

// ── fixtures ──────────────────────────────────────────────────────────────────

async fn insert_book(pool: &SqlitePool) -> Book {
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

async fn insert_commodity(pool: &SqlitePool, book_id: BookId, mnemonic: &str) -> Commodity {
    let c = Commodity {
        id: CommodityId::new(),
        book_id,
        namespace: "CURRENCY".to_string(),
        mnemonic: mnemonic.to_string(),
        name: mnemonic.to_string(),
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

fn make_price(
    book_id: BookId,
    commodity_id: CommodityId,
    currency_id: CommodityId,
    date: NaiveDate,
    value: Decimal,
) -> Price {
    Price {
        id: PriceId::new(),
        book_id,
        commodity_id,
        currency_id,
        date,
        value,
        source: PriceSource::User,
        created_at: Utc::now(),
    }
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[sqlx::test]
async fn insert_and_find_by_id(pool: SqlitePool) {
    let book = insert_book(&pool).await;
    let aapl = insert_commodity(&pool, book.id, "AAPL").await;
    let usd = insert_commodity(&pool, book.id, "USD").await;
    let repo = PriceRepository::new(pool);

    let price = make_price(
        book.id,
        aapl.id,
        usd.id,
        NaiveDate::from_ymd_opt(2024, 1, 15).unwrap(),
        Decimal::new(18250, 2),
    );
    repo.insert(&price).await.unwrap();

    let found = repo.find_by_id(price.id).await.unwrap().unwrap();
    assert_eq!(found.id, price.id);
    assert_eq!(found.commodity_id, price.commodity_id);
    assert_eq!(found.currency_id, price.currency_id);
    assert_eq!(found.date, price.date);
    assert_eq!(found.value, price.value);
    assert_eq!(found.source, price.source);
}

#[sqlx::test]
async fn find_by_id_returns_none_for_missing(pool: SqlitePool) {
    let repo = PriceRepository::new(pool);
    assert!(repo.find_by_id(PriceId::new()).await.unwrap().is_none());
}

#[sqlx::test]
async fn round_trip_preserves_all_sources(pool: SqlitePool) {
    let book = insert_book(&pool).await;
    let aapl = insert_commodity(&pool, book.id, "AAPL").await;
    let usd = insert_commodity(&pool, book.id, "USD").await;
    let repo = PriceRepository::new(pool);

    let sources = [
        PriceSource::User,
        PriceSource::AlphaVantage,
        PriceSource::YahooFinance,
        PriceSource::Import,
        PriceSource::Transaction,
    ];
    for source in sources {
        let mut p = make_price(
            book.id,
            aapl.id,
            usd.id,
            NaiveDate::from_ymd_opt(2024, 1, 15).unwrap(),
            Decimal::new(100, 0),
        );
        p.source = source.clone();
        repo.insert(&p).await.unwrap();
        let found = repo.find_by_id(p.id).await.unwrap().unwrap();
        assert_eq!(found.source, source);
    }
}

#[sqlx::test]
async fn find_by_book_returns_prices_ordered_by_date_desc(pool: SqlitePool) {
    let book = insert_book(&pool).await;
    let aapl = insert_commodity(&pool, book.id, "AAPL").await;
    let usd = insert_commodity(&pool, book.id, "USD").await;
    let repo = PriceRepository::new(pool);

    let dates = [
        NaiveDate::from_ymd_opt(2024, 3, 1).unwrap(),
        NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        NaiveDate::from_ymd_opt(2024, 2, 1).unwrap(),
    ];
    for date in dates {
        repo.insert(&make_price(
            book.id,
            aapl.id,
            usd.id,
            date,
            Decimal::new(100, 0),
        ))
        .await
        .unwrap();
    }

    let found = repo.find_by_book(book.id).await.unwrap();
    assert_eq!(found.len(), 3);
    assert_eq!(found[0].date, NaiveDate::from_ymd_opt(2024, 3, 1).unwrap());
    assert_eq!(found[1].date, NaiveDate::from_ymd_opt(2024, 2, 1).unwrap());
    assert_eq!(found[2].date, NaiveDate::from_ymd_opt(2024, 1, 1).unwrap());
}

#[sqlx::test]
async fn find_by_book_excludes_other_books(pool: SqlitePool) {
    let book_a = insert_book(&pool).await;
    let book_b = insert_book(&pool).await;
    let c_a = insert_commodity(&pool, book_a.id, "AAPL").await;
    let usd_a = insert_commodity(&pool, book_a.id, "USD").await;
    let c_b = insert_commodity(&pool, book_b.id, "GOOG").await;
    let usd_b = insert_commodity(&pool, book_b.id, "USD").await;
    let repo = PriceRepository::new(pool);

    let date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
    repo.insert(&make_price(
        book_a.id,
        c_a.id,
        usd_a.id,
        date,
        Decimal::new(100, 0),
    ))
    .await
    .unwrap();
    repo.insert(&make_price(
        book_b.id,
        c_b.id,
        usd_b.id,
        date,
        Decimal::new(200, 0),
    ))
    .await
    .unwrap();

    let found = repo.find_by_book(book_a.id).await.unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].commodity_id, c_a.id);
}

#[sqlx::test]
async fn latest_before_returns_most_recent_on_or_before(pool: SqlitePool) {
    let book = insert_book(&pool).await;
    let aapl = insert_commodity(&pool, book.id, "AAPL").await;
    let usd = insert_commodity(&pool, book.id, "USD").await;
    let repo = PriceRepository::new(pool);

    let prices = [
        (
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            Decimal::new(18000, 2),
        ),
        (
            NaiveDate::from_ymd_opt(2024, 1, 15).unwrap(),
            Decimal::new(18500, 2),
        ),
        (
            NaiveDate::from_ymd_opt(2024, 2, 1).unwrap(),
            Decimal::new(19000, 2),
        ),
    ];
    for (date, value) in prices {
        repo.insert(&make_price(book.id, aapl.id, usd.id, date, value))
            .await
            .unwrap();
    }

    // Jan 20 → latest on or before is Jan 15
    let found = repo
        .latest_before(
            aapl.id,
            usd.id,
            NaiveDate::from_ymd_opt(2024, 1, 20).unwrap(),
        )
        .await
        .unwrap()
        .unwrap();
    assert_eq!(found.date, NaiveDate::from_ymd_opt(2024, 1, 15).unwrap());
    assert_eq!(found.value, Decimal::new(18500, 2));
}

#[sqlx::test]
async fn latest_before_returns_none_when_no_price_exists(pool: SqlitePool) {
    let book = insert_book(&pool).await;
    let aapl = insert_commodity(&pool, book.id, "AAPL").await;
    let usd = insert_commodity(&pool, book.id, "USD").await;
    let repo = PriceRepository::new(pool);

    // Only a price on Feb 1 — nothing on or before Jan 1
    repo.insert(&make_price(
        book.id,
        aapl.id,
        usd.id,
        NaiveDate::from_ymd_opt(2024, 2, 1).unwrap(),
        Decimal::new(100, 0),
    ))
    .await
    .unwrap();

    let found = repo
        .latest_before(
            aapl.id,
            usd.id,
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        )
        .await
        .unwrap();
    assert!(found.is_none());
}

#[sqlx::test]
async fn latest_before_matches_exact_date(pool: SqlitePool) {
    let book = insert_book(&pool).await;
    let aapl = insert_commodity(&pool, book.id, "AAPL").await;
    let usd = insert_commodity(&pool, book.id, "USD").await;
    let repo = PriceRepository::new(pool);

    let date = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
    repo.insert(&make_price(
        book.id,
        aapl.id,
        usd.id,
        date,
        Decimal::new(18500, 2),
    ))
    .await
    .unwrap();

    let found = repo
        .latest_before(aapl.id, usd.id, date)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(found.date, date);
}

#[sqlx::test]
async fn find_series_returns_chronological_prices(pool: SqlitePool) {
    let book = insert_book(&pool).await;
    let aapl = insert_commodity(&pool, book.id, "AAPL").await;
    let usd = insert_commodity(&pool, book.id, "USD").await;
    let repo = PriceRepository::new(pool);

    let dates = [
        NaiveDate::from_ymd_opt(2024, 3, 1).unwrap(),
        NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        NaiveDate::from_ymd_opt(2024, 2, 1).unwrap(),
    ];
    for date in dates {
        repo.insert(&make_price(
            book.id,
            aapl.id,
            usd.id,
            date,
            Decimal::new(100, 0),
        ))
        .await
        .unwrap();
    }

    let series = repo.find_series(aapl.id, usd.id).await.unwrap();
    assert_eq!(series.len(), 3);
    assert_eq!(series[0].date, NaiveDate::from_ymd_opt(2024, 1, 1).unwrap());
    assert_eq!(series[1].date, NaiveDate::from_ymd_opt(2024, 2, 1).unwrap());
    assert_eq!(series[2].date, NaiveDate::from_ymd_opt(2024, 3, 1).unwrap());
}

#[sqlx::test]
async fn delete_removes_price(pool: SqlitePool) {
    let book = insert_book(&pool).await;
    let aapl = insert_commodity(&pool, book.id, "AAPL").await;
    let usd = insert_commodity(&pool, book.id, "USD").await;
    let repo = PriceRepository::new(pool);

    let price = make_price(
        book.id,
        aapl.id,
        usd.id,
        NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        Decimal::new(100, 0),
    );
    repo.insert(&price).await.unwrap();
    repo.delete(price.id).await.unwrap();
    assert!(repo.find_by_id(price.id).await.unwrap().is_none());
}

#[sqlx::test]
async fn delete_unknown_id_is_not_found(pool: SqlitePool) {
    let repo = PriceRepository::new(pool);
    let err = repo.delete(PriceId::new()).await.unwrap_err();
    assert!(matches!(err, StorageError::NotFound { .. }));
}

#[sqlx::test]
async fn update_changes_date_value_and_source(pool: SqlitePool) {
    let book = insert_book(&pool).await;
    let aapl = insert_commodity(&pool, book.id, "AAPL").await;
    let usd = insert_commodity(&pool, book.id, "USD").await;
    let repo = PriceRepository::new(pool);

    let price = make_price(
        book.id,
        aapl.id,
        usd.id,
        NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        Decimal::new(100, 0),
    );
    repo.insert(&price).await.unwrap();

    let new_date = NaiveDate::from_ymd_opt(2024, 6, 1).unwrap();
    let new_value = Decimal::new(20000, 2);
    repo.update(
        price.id,
        new_date,
        new_value,
        PriceSource::Import,
        Utc::now(),
    )
    .await
    .unwrap();

    let found = repo.find_by_id(price.id).await.unwrap().unwrap();
    assert_eq!(found.date, new_date);
    assert_eq!(found.value, new_value);
    assert_eq!(found.source, PriceSource::Import);
}

#[sqlx::test]
async fn update_unknown_id_is_not_found(pool: SqlitePool) {
    let repo = PriceRepository::new(pool);
    let err = repo
        .update(
            PriceId::new(),
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            Decimal::new(100, 0),
            PriceSource::User,
            Utc::now(),
        )
        .await
        .unwrap_err();
    assert!(matches!(err, StorageError::NotFound { .. }));
}

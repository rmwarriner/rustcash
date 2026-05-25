use anyhow::{bail, Context as _};
use rustcash_core::ids::BookId;
use rustcash_storage::{
    open_sqlite, run_migrations,
    repositories::books::BookRepository,
    SqlitePool,
};

pub struct Ctx {
    pub pool:    SqlitePool,
    pub book_id: BookId,
}

impl Ctx {
    pub async fn open(db_path: &str, book_arg: Option<&str>) -> anyhow::Result<Self> {
        let url = if db_path.starts_with("sqlite:") {
            db_path.to_string()
        } else {
            format!("sqlite:{db_path}")
        };

        let pool = open_sqlite(&url).await.context("opening database")?;
        run_migrations(&pool).await.context("running migrations")?;

        let book_id = if let Some(raw) = book_arg {
            raw.parse::<uuid::Uuid>()
                .with_context(|| format!("invalid book ID: {raw}"))?
                .into()
        } else {
            let books = BookRepository::new(pool.clone()).find_all().await?;
            match books.into_iter().next() {
                Some(b) => b.id,
                None => bail!(
                    "no books found — run `rustcash database init` to create one"
                ),
            }
        };

        Ok(Self { pool, book_id })
    }
}

/// Default DB file path when neither --file nor RUSTCASH_DB is set.
pub fn default_db_path() -> String {
    dirs::data_local_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("rustcash")
        .join("rustcash.db")
        .to_string_lossy()
        .into_owned()
}

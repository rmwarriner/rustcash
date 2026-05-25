use anyhow::Result;
use clap::{Parser, Subcommand};
use rustcash_cli::{
    commands::{
        account::{
            CreateAccountArgs, cmd_balance, cmd_create, cmd_delete, cmd_list, cmd_rename, cmd_show,
        },
        database::{cmd_backup, cmd_init, cmd_purge, cmd_seed, cmd_status},
    },
    context::{Ctx, default_db_path},
};

#[derive(Parser)]
#[command(
    name = "rustcash",
    about = "Modern accounting from the command line",
    version
)]
struct Cli {
    /// SQLite database file (env: RUSTCASH_DB)
    #[arg(long, short = 'f', env = "RUSTCASH_DB", global = true)]
    file: Option<String>,

    /// Book ID to operate on (defaults to first active book)
    #[arg(long, global = true)]
    book: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Manage the database (init, status, backup, seed, purge)
    Database {
        #[command(subcommand)]
        cmd: DatabaseCmd,
    },
    /// List, create, and inspect accounts
    Account {
        #[command(subcommand)]
        cmd: AccountCmd,
    },
    /// List, create, and delete transactions
    Transaction {
        #[command(subcommand)]
        cmd: TransactionCmd,
    },
    /// Import transactions from a file
    Import {
        /// Path to the file to import
        file: std::path::PathBuf,
        /// Format override (csv, ofx, qif, gnucash)
        #[arg(long)]
        format: Option<String>,
    },
    /// Render a report
    Report {
        #[command(subcommand)]
        cmd: ReportCmd,
    },
    /// Start the API server
    Serve {
        #[arg(long, default_value = "127.0.0.1:8080")]
        bind: String,
    },
}

#[derive(Subcommand)]
enum DatabaseCmd {
    /// Create the database, run migrations, and set up an initial book
    Init {
        /// Book name
        #[arg(long, default_value = "My Finances")]
        name: String,
        /// Default currency code (ISO 4217)
        #[arg(long, default_value = "USD")]
        currency: String,
        /// Create a book even if one already exists
        #[arg(long)]
        force: bool,
    },
    /// Show database status and book summary
    Status,
    /// Copy the database file to a backup location
    Backup {
        /// Output path (defaults to <db>.bak-<timestamp>)
        #[arg(long)]
        output: Option<String>,
    },
    /// Populate a book with a starter chart of accounts
    Seed {
        /// Account template to use
        #[arg(long, default_value = "standard", value_parser = ["standard", "minimal"])]
        template: String,
    },
    /// Hard-delete soft-deleted records past their retention period
    Purge {
        /// Delete records soft-deleted more than this many days ago
        #[arg(long, default_value_t = 90)]
        older_than: u32,
        /// Show what would be deleted without making changes
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(Subcommand)]
enum AccountCmd {
    /// List all accounts
    List {
        #[arg(long, default_value = "table", value_parser = ["table", "json", "csv"])]
        format: String,
    },
    /// Show account detail
    Show { id: String },
    /// Show account balance
    Balance {
        id: String,
        /// Date to compute balance as of (YYYY-MM-DD; defaults to today)
        #[arg(long)]
        as_of: Option<String>,
    },
    /// Create a new account
    Create {
        /// Account name
        name: String,
        /// Account type (asset, cash, bank, credit_card, investment, mutual_fund,
        ///   liability, long_term_liability, equity, opening_balance, retained_earnings,
        ///   income, expense, receivable, payable)
        #[arg(long, short = 't')]
        r#type: String,
        /// Parent account ID (omit for a root account)
        #[arg(long, short = 'p')]
        parent: Option<String>,
        /// Commodity/currency ID (defaults to the book's default currency)
        #[arg(long, short = 'c')]
        currency: Option<String>,
        /// Optional description
        #[arg(long, short = 'd')]
        description: Option<String>,
        /// Mark account as a placeholder (container only, no direct transactions)
        #[arg(long)]
        placeholder: bool,
        /// Hide account from normal views
        #[arg(long)]
        hidden: bool,
    },
    /// Rename an account (cascades full_name to all descendants)
    Rename {
        /// Account ID
        id: String,
        /// New name
        name: String,
    },
    /// Soft-delete an account
    Delete {
        /// Account ID
        id: String,
    },
}

#[derive(Subcommand)]
enum TransactionCmd {
    /// List transactions
    List {
        #[arg(long)]
        account: Option<String>,
        #[arg(long)]
        from: Option<String>,
        #[arg(long)]
        to: Option<String>,
        #[arg(long, default_value = "table", value_parser = ["table", "json", "csv"])]
        format: String,
    },
    /// Show a transaction
    Show { id: String },
}

#[derive(Subcommand)]
enum ReportCmd {
    /// List available reports
    List,
    /// Render a report
    Render {
        id: String,
        #[arg(long)]
        from: Option<String>,
        #[arg(long)]
        to: Option<String>,
        #[arg(long, default_value = "html", value_parser = ["html", "csv", "json"])]
        format: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("rustcash=info".parse().unwrap()),
        )
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();
    let db_path = cli.file.unwrap_or_else(default_db_path);

    match cli.command {
        Commands::Database { cmd } => {
            // Database commands act on the file directly — no Ctx needed.
            match cmd {
                DatabaseCmd::Init {
                    name,
                    currency,
                    force,
                } => {
                    cmd_init(&db_path, &name, &currency, force).await?;
                }
                DatabaseCmd::Status => {
                    cmd_status(&db_path).await?;
                }
                DatabaseCmd::Backup { output } => {
                    cmd_backup(&db_path, output.as_deref()).await?;
                }
                DatabaseCmd::Seed { template } => {
                    cmd_seed(&db_path, cli.book.as_deref(), &template).await?;
                }
                DatabaseCmd::Purge {
                    older_than,
                    dry_run,
                } => {
                    cmd_purge(&db_path, older_than, dry_run).await?;
                }
            }
        }

        Commands::Account { cmd } => {
            let ctx = Ctx::open(&db_path, cli.book.as_deref()).await?;
            match cmd {
                AccountCmd::List { format } => {
                    cmd_list(&ctx.pool, ctx.book_id, &format).await?;
                }
                AccountCmd::Show { id } => {
                    cmd_show(&ctx.pool, ctx.book_id, &id).await?;
                }
                AccountCmd::Balance { id, as_of } => {
                    cmd_balance(&ctx.pool, &id, ctx.book_id, as_of.as_deref()).await?;
                }
                AccountCmd::Create {
                    name,
                    r#type,
                    parent,
                    currency,
                    description,
                    placeholder,
                    hidden,
                } => {
                    cmd_create(
                        &ctx.pool,
                        ctx.book_id,
                        CreateAccountArgs {
                            name: &name,
                            type_str: &r#type,
                            parent_id: parent.as_deref(),
                            commodity_id: currency.as_deref(),
                            description: description.as_deref(),
                            placeholder,
                            hidden,
                        },
                    )
                    .await?;
                }
                AccountCmd::Rename { id, name } => {
                    cmd_rename(&ctx.pool, ctx.book_id, &id, &name).await?;
                }
                AccountCmd::Delete { id } => {
                    cmd_delete(&ctx.pool, ctx.book_id, &id).await?;
                }
            }
        }

        Commands::Serve { bind } => {
            println!("Starting API server on {bind} — not yet implemented");
        }
        Commands::Transaction { cmd } => match cmd {
            TransactionCmd::List {
                account,
                from,
                to,
                format,
            } => {
                println!(
                    "transaction list account={account:?} from={from:?} to={to:?} format={format} — not yet implemented"
                )
            }
            TransactionCmd::Show { id } => println!("transaction show {id} — not yet implemented"),
        },
        Commands::Import { file, format } => {
            println!("import {file:?} format={format:?} — not yet implemented")
        }
        Commands::Report { cmd } => match cmd {
            ReportCmd::List => println!("report list — not yet implemented"),
            ReportCmd::Render {
                id,
                from,
                to,
                format,
            } => {
                println!(
                    "report render {id} from={from:?} to={to:?} format={format} — not yet implemented"
                )
            }
        },
    }

    Ok(())
}

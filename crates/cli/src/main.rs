use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "rustcash", about = "Modern accounting from the command line", version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
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
        #[arg(long)]
        as_of: Option<String>,
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
    let cli = Cli::parse();

    match cli.command {
        Commands::Serve { bind } => {
            println!("Starting API server on {bind} — not yet implemented");
        }
        Commands::Account { cmd } => match cmd {
            AccountCmd::List { format } => println!("account list (format={format}) — not yet implemented"),
            AccountCmd::Show { id } => println!("account show {id} — not yet implemented"),
            AccountCmd::Balance { id, as_of } => println!("account balance {id} as_of={as_of:?} — not yet implemented"),
        },
        Commands::Transaction { cmd } => match cmd {
            TransactionCmd::List { account, from, to, format } => {
                println!("transaction list account={account:?} from={from:?} to={to:?} format={format} — not yet implemented")
            }
            TransactionCmd::Show { id } => println!("transaction show {id} — not yet implemented"),
        },
        Commands::Import { file, format } => {
            println!("import {file:?} format={format:?} — not yet implemented")
        }
        Commands::Report { cmd } => match cmd {
            ReportCmd::List => println!("report list — not yet implemented"),
            ReportCmd::Render { id, from, to, format } => {
                println!("report render {id} from={from:?} to={to:?} format={format} — not yet implemented")
            }
        },
    }

    Ok(())
}

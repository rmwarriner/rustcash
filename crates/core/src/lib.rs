pub mod account;
pub mod book;
pub mod budget;
pub mod commodity;
pub mod error;
pub mod ids;
pub mod transaction;

// Flat re-exports for ergonomic use: `use rustcash_core::prelude::*`
pub mod prelude {
    pub use crate::account::{Account, AccountType, RootType};
    pub use crate::book::Book;
    pub use crate::budget::{Budget, BudgetAllocation, BudgetPeriod, BudgetPeriodType};
    pub use crate::commodity::{Commodity, Price, PriceSource};
    pub use crate::error::CoreError;
    pub use crate::ids::{
        AccountId, BookId, BudgetId, CommodityId, LotId, PriceId, SplitId, TransactionId, UserId,
    };
    pub use crate::transaction::{ReconcileState, Split, Transaction};
}

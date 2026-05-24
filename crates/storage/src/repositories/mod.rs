//! Repository modules — one per aggregate root.
//!
//! Each repository is a thin async CRUD layer. Business logic lives in `engine`.

pub mod accounts;
pub mod books;
pub mod commodities;
pub mod prices;
pub mod transactions;

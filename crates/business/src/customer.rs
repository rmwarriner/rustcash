use serde::{Deserialize, Serialize};
use uuid::Uuid;
use rustcash_core::ids::BookId;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CustomerId(pub Uuid);

impl CustomerId {
    pub fn new() -> Self { Self(Uuid::new_v4()) }
}

impl Default for CustomerId {
    fn default() -> Self { Self::new() }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Customer {
    pub id:        CustomerId,
    pub book_id:   BookId,
    pub name:      String,
    pub company:   Option<String>,
    pub email:     Option<String>,
    pub phone:     Option<String>,
    pub address:   Option<String>,
    pub notes:     Option<String>,
    pub active:    bool,
}

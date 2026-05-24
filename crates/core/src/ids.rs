use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

macro_rules! id_newtype {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        pub struct $name(pub Uuid);

        impl $name {
            pub fn new() -> Self {
                Self(Uuid::new_v4())
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl From<Uuid> for $name {
            fn from(u: Uuid) -> Self {
                Self(u)
            }
        }

        impl From<$name> for Uuid {
            fn from(id: $name) -> Uuid {
                id.0
            }
        }
    };
}

id_newtype!(BookId,        "Identifies a book (a single accounting database).");
id_newtype!(AccountId,     "Identifies an account in the account tree.");
id_newtype!(TransactionId, "Identifies a double-entry transaction.");
id_newtype!(SplitId,       "Identifies one leg of a transaction.");
id_newtype!(CommodityId,   "Identifies a commodity or currency.");
id_newtype!(PriceId,       "Identifies a commodity price quote.");
id_newtype!(BudgetId,      "Identifies a budget.");
id_newtype!(LotId,         "Identifies a cost-basis lot (investments).");

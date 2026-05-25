//! Account management: creation, renaming (with full_name cascade), soft-delete.

use chrono::Utc;
use rustcash_core::{account::Account, ids::AccountId};
use rustcash_storage::{repositories::accounts::AccountRepository, SqlitePool};

use crate::EngineError;

pub struct AccountService {
    pool: SqlitePool,
}

impl AccountService {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Insert a new account, validating that its parent exists (if set).
    pub async fn create(&self, account: &Account) -> Result<(), EngineError> {
        let repo = AccountRepository::new(self.pool.clone());

        if let Some(parent_id) = account.parent_id {
            repo.find_by_id(parent_id)
                .await?
                .ok_or_else(|| EngineError::AccountNotFound { id: parent_id.to_string() })?;
        }

        repo.insert(account).await?;
        Ok(())
    }

    /// Rename an account and cascade the new full_name down to all descendants.
    ///
    /// Descendants' full_names are rebuilt as `{parent_full_name}:{child.name}`.
    pub async fn rename(
        &self,
        id: AccountId,
        name: String,
        full_name: String,
    ) -> Result<(), EngineError> {
        let repo = AccountRepository::new(self.pool.clone());

        let mut account = repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| EngineError::AccountNotFound { id: id.to_string() })?;

        account.name      = name;
        account.full_name = full_name.clone();
        account.modified_at = Utc::now();
        repo.update(&account).await?;

        // Iterative BFS to cascade full_name to all descendants.
        // Each queue entry is (parent_id, parent_full_name).
        let mut queue: Vec<(AccountId, String)> = vec![(id, full_name)];
        while let Some((parent_id, parent_full_name)) = queue.pop() {
            for mut child in repo.find_children(parent_id).await? {
                let new_full_name = format!("{}:{}", parent_full_name, child.name);
                child.full_name  = new_full_name.clone();
                child.modified_at = Utc::now();
                repo.update(&child).await?;
                queue.push((child.id, new_full_name));
            }
        }

        Ok(())
    }

    /// Soft-delete an account (sets `deleted_at`, excludes from active lists).
    pub async fn soft_delete(&self, id: AccountId) -> Result<(), EngineError> {
        AccountRepository::new(self.pool.clone())
            .soft_delete(id, Utc::now())
            .await?;
        Ok(())
    }
}

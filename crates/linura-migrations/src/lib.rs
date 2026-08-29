#![forbid(unsafe_code)]

use std::collections::BTreeSet;
use std::fmt::{Display, Formatter};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum MigrationScope {
    System,
    User,
    Intent,
    Graph,
    Profile,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MigrationDescriptor {
    pub id: String,
    pub introduced_in: String,
    pub scope: MigrationScope,
    pub reversible: bool,
    pub requires_snapshot: bool,
}

impl MigrationDescriptor {
    pub fn validate(&self) -> Result<(), MigrationError> {
        if self.id.trim().is_empty() {
            return Err(MigrationError::InvalidDescriptor("migration id cannot be empty"));
        }
        if self.introduced_in.trim().is_empty() {
            return Err(MigrationError::InvalidDescriptor("introduced_in cannot be empty"));
        }
        Ok(())
    }
}

pub trait Migration {
    fn descriptor(&self) -> &MigrationDescriptor;
    fn precondition(&self) -> Result<bool, MigrationError>;
    fn apply(&self) -> Result<(), MigrationError>;
    fn verify(&self) -> Result<(), MigrationError>;
    fn rollback(&self) -> Result<(), MigrationError> {
        Err(MigrationError::ManualRecoveryRequired(self.descriptor().id.clone()))
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MigrationLedger {
    applied: BTreeSet<String>,
}

impl MigrationLedger {
    #[must_use]
    pub fn is_applied(&self, id: &str) -> bool {
        self.applied.contains(id)
    }

    pub fn mark_applied(&mut self, id: impl Into<String>) {
        self.applied.insert(id.into());
    }
}

#[derive(Debug)]
pub struct MigrationRunner {
    ledger: MigrationLedger,
}

impl MigrationRunner {
    #[must_use]
    pub fn new(ledger: MigrationLedger) -> Self {
        Self { ledger }
    }

    pub fn run(&mut self, migration: &dyn Migration) -> Result<MigrationOutcome, MigrationError> {
        let descriptor = migration.descriptor();
        descriptor.validate()?;
        if self.ledger.is_applied(&descriptor.id) {
            return Ok(MigrationOutcome::AlreadyApplied);
        }
        if !migration.precondition()? {
            return Ok(MigrationOutcome::NotApplicable);
        }
        migration.apply()?;
        if let Err(error) = migration.verify() {
            if descriptor.reversible {
                migration.rollback()?;
            }
            return Err(error);
        }
        self.ledger.mark_applied(descriptor.id.clone());
        Ok(MigrationOutcome::Applied)
    }

    #[must_use]
    pub fn ledger(&self) -> &MigrationLedger {
        &self.ledger
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MigrationOutcome {
    Applied,
    AlreadyApplied,
    NotApplicable,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MigrationError {
    InvalidDescriptor(&'static str),
    Operation(String),
    ManualRecoveryRequired(String),
}

impl Display for MigrationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidDescriptor(message) | Self::Operation(message) => f.write_str(message),
            Self::ManualRecoveryRequired(id) => write!(f, "migration {id} requires manual recovery"),
        }
    }
}

impl std::error::Error for MigrationError {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[derive(Debug)]
    struct TestMigration {
        descriptor: MigrationDescriptor,
        applications: Cell<u32>,
    }

    impl Migration for TestMigration {
        fn descriptor(&self) -> &MigrationDescriptor { &self.descriptor }
        fn precondition(&self) -> Result<bool, MigrationError> { Ok(true) }
        fn apply(&self) -> Result<(), MigrationError> { self.applications.set(self.applications.get() + 1); Ok(()) }
        fn verify(&self) -> Result<(), MigrationError> { Ok(()) }
    }

    #[test]
    fn migrations_are_idempotent_through_the_ledger() {
        let migration = TestMigration {
            descriptor: MigrationDescriptor {
                id: "0001-test".into(), introduced_in: "0.0.1".into(), scope: MigrationScope::System,
                reversible: false, requires_snapshot: true,
            },
            applications: Cell::new(0),
        };
        let mut runner = MigrationRunner::new(MigrationLedger::default());
        assert_eq!(runner.run(&migration), Ok(MigrationOutcome::Applied));
        assert_eq!(runner.run(&migration), Ok(MigrationOutcome::AlreadyApplied));
        assert_eq!(migration.applications.get(), 1);
    }
}

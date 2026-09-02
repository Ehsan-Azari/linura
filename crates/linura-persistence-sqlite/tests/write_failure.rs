#![forbid(unsafe_code)]

#[cfg(unix)]
mod unix {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    use linura_core::{
        CapabilityId, PlanId, PolicyId, PolicyRevisionId, PrincipalId, ProviderId, RequestId,
        ResourceId, RiskClass, ValidationError,
    };
    use linura_persistence_sqlite::SqliteTransactionStore;
    use linura_transaction::{
        AuthorityBinding, AuthorizationBasis, ContentDigest, TransactionStore,
        TransactionStoreError, TransactionValidationError, digest_bytes,
    };

    static NEXT_DB: AtomicU64 = AtomicU64::new(0);

    struct TestDirectory {
        path: PathBuf,
    }

    impl TestDirectory {
        fn new() -> Self {
            let sequence = NEXT_DB.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "linura-v04-write-failure-{}-{sequence}",
                std::process::id()
            ));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir(&path).unwrap_or_else(|error| unreachable!("{error}"));
            Self { path }
        }

        fn db(&self) -> PathBuf {
            self.path.join("authority.db")
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::set_permissions(&self.path, fs::Permissions::from_mode(0o700));
            let _ = fs::set_permissions(self.db(), fs::Permissions::from_mode(0o600));
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn id<T>(value: Result<T, ValidationError>) -> T {
        value.unwrap_or_else(|error| unreachable!("{error}"))
    }

    fn digest(value: &str) -> ContentDigest {
        digest_bytes("linura.v04-write-failure-test.v1", value.as_bytes())
    }

    fn binding() -> AuthorityBinding {
        AuthorityBinding::try_new(
            id(PrincipalId::new("uid:1000")),
            id(RequestId::new("request:write-failure")),
            id(PlanId::new("request:write-failure")),
            digest("request"),
            digest("precondition"),
            digest("observation"),
            id(ProviderId::new("systemd")),
            id(ResourceId::new("systemd:unit:write-failure.service")),
            id(CapabilityId::new("systemd.unit.observe")),
            id(PolicyId::new("policy:qualification")),
            id(PolicyRevisionId::new("policy:qualification:v1")),
            RiskClass::SecuritySensitive,
            "risk-policy:v0.4:qualification",
            vec!["qualification.no-external-effect".into()],
            digest("review"),
            AuthorizationBasis::PolicyAllow,
        )
        .unwrap_or_else(|error: TransactionValidationError| unreachable!("{error}"))
    }

    fn sidecar(path: &Path, suffix: &str) -> PathBuf {
        PathBuf::from(format!("{}{suffix}", path.display()))
    }

    #[test]
    fn write_denial_fails_closed_without_rewriting_durable_history() {
        let directory = TestDirectory::new();
        let db = directory.db();
        let binding = binding();
        {
            let mut store =
                SqliteTransactionStore::open(&db).unwrap_or_else(|error| unreachable!("{error}"));
            store
                .prepare(&binding)
                .unwrap_or_else(|error| unreachable!("{error}"));
            store
                .integrity_check()
                .unwrap_or_else(|error| unreachable!("{error}"));
        }

        let before = fs::read(&db).unwrap_or_else(|error| unreachable!("{error}"));
        let _ = fs::remove_file(sidecar(&db, "-wal"));
        let _ = fs::remove_file(sidecar(&db, "-shm"));
        fs::set_permissions(&db, fs::Permissions::from_mode(0o400))
            .unwrap_or_else(|error| unreachable!("{error}"));
        fs::set_permissions(&directory.path, fs::Permissions::from_mode(0o500))
            .unwrap_or_else(|error| unreachable!("{error}"));

        let result = SqliteTransactionStore::open(&db);
        assert!(matches!(
            result,
            Err(TransactionStoreError::Storage(_))
                | Err(TransactionStoreError::UnsupportedSchema(_))
        ));

        fs::set_permissions(&directory.path, fs::Permissions::from_mode(0o700))
            .unwrap_or_else(|error| unreachable!("{error}"));
        fs::set_permissions(&db, fs::Permissions::from_mode(0o600))
            .unwrap_or_else(|error| unreachable!("{error}"));
        let after = fs::read(&db).unwrap_or_else(|error| unreachable!("{error}"));
        assert_eq!(before, after);

        let reopened =
            SqliteTransactionStore::open(&db).unwrap_or_else(|error| unreachable!("{error}"));
        reopened
            .integrity_check()
            .unwrap_or_else(|error| unreachable!("{error}"));
        assert_eq!(
            reopened
                .snapshot(&binding.transaction_id())
                .unwrap_or_else(|error| unreachable!("{error}"))
                .binding_digest,
            *binding.digest()
        );
    }
}

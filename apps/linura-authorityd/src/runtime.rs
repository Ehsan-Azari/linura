use std::collections::BTreeMap;
use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

use linura_control::{
    MANAGED_SYSTEMD_CAPABILITY, MANAGED_SYSTEMD_INTENT_ORIGIN, MANAGED_SYSTEMD_PROVIDER,
    ManagedLifecycleControl, PlanPreviewControl, managed_request_id,
};
use linura_core::{CapabilityId, IntentId, ProviderId, RequestId, SemanticReason};
use linura_dbus::{Authority1Context, Authority1Handler, Authority1ManagedRequest};
use linura_executor_systemd::{ManagedActiveState, ManagedUnitName};
use linura_linux_observation::SystemdObserver;
use linura_observation_control::ObservationCoordinator;
use linura_persistence_sqlite::{SqliteIntegrityKey, SqliteTransactionStore};
use linura_protocol::PlanDesiredStateRequest;
use linura_transaction::TransactionAuthorityKey;

use crate::systemd_adapter::{FreshSystemdVerifier, SystemdExecutorClient};

const DEFAULT_STATE_DIR: &str = "/var/lib/linura-authority";
const STATE_DIR_ENV: &str = "LINURA_AUTHORITY_STATE_DIR";
const AUTHORITY_KEY_FILE: &str = "transaction-authority.key";
const INTEGRITY_KEY_FILE: &str = "sqlite-integrity.key";
const DATABASE_FILE: &str = "authority.sqlite3";
const SECRET_BYTES: usize = 32;
const MAX_REASON_BYTES: usize = 1024;

#[derive(Debug)]
pub(crate) struct ManagedRuntime {
    control: ManagedLifecycleControl<SqliteTransactionStore>,
    executor: SystemdExecutorClient,
    verifier: FreshSystemdVerifier,
}

impl ManagedRuntime {
    pub(crate) fn open(state_dir: &Path) -> Result<Self, String> {
        prepare_state_dir(state_dir)?;
        let authority_bytes = load_or_create_secret(&state_dir.join(AUTHORITY_KEY_FILE))?;
        let integrity_bytes = load_or_create_secret(&state_dir.join(INTEGRITY_KEY_FILE))?;

        let authority_key = TransactionAuthorityKey::new(authority_bytes)
            .map_err(|error| format!("invalid transaction authority key: {error}"))?;
        let (authority_signer, authority_verifier) = authority_key.split();
        let integrity_key = SqliteIntegrityKey::new(integrity_bytes)
            .map_err(|error| format!("invalid SQLite integrity key: {error}"))?;
        let store = SqliteTransactionStore::open(
            state_dir.join(DATABASE_FILE),
            authority_verifier,
            integrity_key,
        )
        .map_err(|error| format!("cannot open durable authority store: {error}"))?;

        let control_observer = SystemdObserver::connect()
            .map_err(|error| format!("cannot connect authoritative systemd observer: {error}"))?;
        let mut coordinator = ObservationCoordinator::new();
        coordinator
            .register_observer(Box::new(control_observer))
            .map_err(|error| format!("cannot register authoritative systemd observer: {error}"))?;
        let previews = PlanPreviewControl::new(coordinator);
        let control = ManagedLifecycleControl::new(previews, store, authority_signer)
            .map_err(|error| error.to_string())?;

        Ok(Self {
            control,
            executor: SystemdExecutorClient::connect()?,
            verifier: FreshSystemdVerifier::connect()?,
        })
    }
}

impl Authority1Handler for ManagedRuntime {
    fn converge_systemd_active_state(
        &mut self,
        context: Authority1Context,
        request: Authority1ManagedRequest,
    ) -> Result<linura_control::ManagedMutationReceipt, String> {
        let request = managed_request(&request)?;
        let Self {
            control,
            executor,
            verifier,
        } = self;
        control
            .converge_systemd_active_state(
                context.principal,
                context.actor,
                request,
                &context.approval,
                executor,
                verifier,
            )
            .map_err(|error| error.to_string())
    }
}

fn managed_request(wire: &Authority1ManagedRequest) -> Result<PlanDesiredStateRequest, String> {
    if wire.reason.is_empty()
        || wire.reason.len() > MAX_REASON_BYTES
        || wire.reason.chars().any(char::is_control)
    {
        return Err("reason must be 1..1024 bytes without control characters".into());
    }
    let unit = ManagedUnitName::parse(&wire.unit).map_err(|error| error.to_string())?;
    let state = ManagedActiveState::parse(&wire.desired_active_state).map_err(str::to_owned)?;
    let resource = unit.resource_id().map_err(|error| error.to_string())?;
    let mut request = PlanDesiredStateRequest {
        request_id: RequestId::new("request:v06:pending").map_err(|error| error.to_string())?,
        provider: ProviderId::new(MANAGED_SYSTEMD_PROVIDER).map_err(|error| error.to_string())?,
        resource,
        observation_capability: CapabilityId::new(MANAGED_SYSTEMD_CAPABILITY)
            .map_err(|error| error.to_string())?,
        reason: SemanticReason {
            summary: wire.reason.clone(),
            intent_ids: vec![
                IntentId::new(MANAGED_SYSTEMD_INTENT_ORIGIN)
                    .map_err(|error| error.to_string())?,
            ],
            requirement_ids: vec![],
            capability_ids: vec![],
        },
        desired_state: BTreeMap::from([(
            "active_state".to_owned(),
            state.as_str().to_owned(),
        )]),
    };
    request.request_id =
        managed_request_id(&wire.operation_id, &request).map_err(|error| error.to_string())?;
    Ok(request)
}

fn prepare_state_dir(path: &Path) -> Result<(), String> {
    fs::create_dir_all(path).map_err(|error| format!("cannot create authority state dir: {error}"))?;
    let metadata = fs::metadata(path).map_err(|error| error.to_string())?;
    if !metadata.is_dir() {
        return Err("authority state path is not a directory".into());
    }
    let mode = metadata.permissions().mode() & 0o777;
    if mode & 0o077 != 0 {
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))
            .map_err(|error| format!("cannot harden authority state directory: {error}"))?;
    }
    Ok(())
}

fn load_or_create_secret(path: &Path) -> Result<Vec<u8>, String> {
    match fs::read(path) {
        Ok(bytes) => {
            validate_secret_file(path, &bytes)?;
            Ok(bytes)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => create_secret(path),
        Err(error) => Err(format!("cannot read {}: {error}", path.display())),
    }
}

fn validate_secret_file(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if bytes.len() != SECRET_BYTES || bytes.iter().all(|byte| *byte == 0) {
        return Err(format!("{} is not a non-zero 256-bit secret", path.display()));
    }
    let metadata = fs::metadata(path).map_err(|error| error.to_string())?;
    if metadata.permissions().mode() & 0o077 != 0 {
        return Err(format!("{} permissions are broader than 0600", path.display()));
    }
    Ok(())
}

fn create_secret(path: &Path) -> Result<Vec<u8>, String> {
    let mut random = File::open("/dev/urandom")
        .map_err(|error| format!("cannot open kernel random source: {error}"))?;
    let mut bytes = vec![0_u8; SECRET_BYTES];
    random
        .read_exact(&mut bytes)
        .map_err(|error| format!("cannot read kernel random source: {error}"))?;
    if bytes.iter().all(|byte| *byte == 0) {
        return Err("kernel random source returned an invalid all-zero secret".into());
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .map_err(|error| format!("cannot create {}: {error}", path.display()))?;
    file.write_all(&bytes)
        .and_then(|()| file.sync_all())
        .map_err(|error| format!("cannot durably provision {}: {error}", path.display()))?;
    validate_secret_file(path, &bytes)?;
    Ok(bytes)
}

pub(crate) fn state_dir() -> PathBuf {
    env::var_os(STATE_DIR_ENV)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_STATE_DIR))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wire(unit: &str, state: &str) -> Authority1ManagedRequest {
        Authority1ManagedRequest {
            operation_id: "qualification-operation".into(),
            unit: unit.into(),
            desired_active_state: state.into(),
            reason: "qualify exact managed request".into(),
        }
    }

    #[test]
    fn request_builder_retains_exact_v06_boundary_and_origin() {
        let request = managed_request(&wire("linura-managed-example.service", "active"))
            .unwrap_or_else(|error| unreachable!("{error}"));
        assert_eq!(request.provider.as_str(), MANAGED_SYSTEMD_PROVIDER);
        assert_eq!(
            request.observation_capability.as_str(),
            MANAGED_SYSTEMD_CAPABILITY
        );
        assert_eq!(
            request.resource.as_str(),
            "systemd:unit:linura-managed-example.service"
        );
        assert_eq!(
            request.desired_state.get("active_state").map(String::as_str),
            Some("active")
        );
        assert_eq!(request.reason.intent_ids.len(), 1);
        assert_eq!(
            request.reason.intent_ids[0].as_str(),
            MANAGED_SYSTEMD_INTENT_ORIGIN
        );
    }

    #[test]
    fn request_builder_rejects_scope_widening() {
        assert!(managed_request(&wire("sshd.service", "active")).is_err());
        assert!(managed_request(&wire("linura-managed-example.service", "failed")).is_err());
    }
}

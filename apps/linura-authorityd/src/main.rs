#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::env;
use std::error::Error;
use std::fmt::{Debug, Formatter};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};

use linura_control::{
    AuthenticatedPrincipal, AuthorizedEffect, AuthorizedEffectExecutor, IndependentManagedVerifier,
    MANAGED_SYSTEMD_CAPABILITY, MANAGED_SYSTEMD_OPERATION, MANAGED_SYSTEMD_PROVIDER,
    ManagedLifecycleControl, ManagedMutationReceipt, PlanPreviewControl, TrustedHumanApproval,
    managed_request_id,
};
use linura_core::{
    Actor, ActorId, ActorKind, CapabilityId, PrincipalId, ProviderId, RequestId, SemanticReason,
};
use linura_executor_systemd::{
    INTERFACE_NAME as EXECUTOR_INTERFACE, ManagedActiveState, ManagedUnitName,
    OBJECT_PATH as EXECUTOR_OBJECT, SERVICE_NAME as EXECUTOR_SERVICE, managed_active_state_effect,
};
use linura_linux_observation::SystemdObserver;
use linura_observation_control::ObservationCoordinator;
use linura_persistence_sqlite::{SqliteIntegrityKey, SqliteTransactionStore};
use linura_protocol::PlanDesiredStateRequest;
use linura_provider_sdk::{
    ComponentDigest, EffectDescriptor, ExecutionDisposition, ExecutionOutcome,
    IndependentVerifier as _, Observer as _, VerificationOutcome,
};
use linura_transaction::TransactionAuthorityKey;
use linura_verifier_systemd::{SystemdActiveStateExpectation, SystemdActiveStateVerifier};
use zbus::message::Header;

const SERVICE_NAME: &str = "org.linura.Authority1";
const OBJECT_PATH: &str = "/org/linura/Authority1";
const HUMAN_ACTION_ID: &str = "org.linura.authority.manage-systemd-active-state";
const DEFAULT_STATE_DIR: &str = "/var/lib/linura-authority";
const STATE_DIR_ENV: &str = "LINURA_AUTHORITY_STATE_DIR";
const AUTHORITY_KEY_FILE: &str = "transaction-authority.key";
const INTEGRITY_KEY_FILE: &str = "sqlite-integrity.key";
const DATABASE_FILE: &str = "authority.sqlite3";
const SECRET_BYTES: usize = 32;
const MAX_REASON_BYTES: usize = 1024;

type ReceiptWire = (
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    bool,
    Vec<String>,
);

type ExecutorOutcomeWire = (String, String, String);

#[derive(Clone, Debug, Eq, PartialEq)]
struct CallerCredentials {
    uid: u32,
    pid: u32,
    unique_bus_name: String,
}

#[derive(Debug)]
struct ManagedRuntime {
    control: ManagedLifecycleControl<SqliteTransactionStore>,
    executor: SystemdExecutorClient,
    verifier: FreshSystemdVerifier,
}

impl ManagedRuntime {
    fn open(state_dir: &Path) -> Result<Self, String> {
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

    fn converge(
        &mut self,
        principal: AuthenticatedPrincipal,
        actor: Actor,
        request: PlanDesiredStateRequest,
        approval: &TrustedHumanApproval,
    ) -> Result<ManagedMutationReceipt, String> {
        let Self {
            control,
            executor,
            verifier,
        } = self;
        control
            .converge_systemd_active_state(principal, actor, request, approval, executor, verifier)
            .map_err(|error| error.to_string())
    }
}

struct SystemdExecutorClient {
    connection: zbus::blocking::Connection,
}

impl Debug for SystemdExecutorClient {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SystemdExecutorClient")
            .finish_non_exhaustive()
    }
}

impl SystemdExecutorClient {
    fn connect() -> Result<Self, String> {
        zbus::blocking::Connection::system()
            .map(|connection| Self { connection })
            .map_err(|error| format!("cannot connect to system bus for executor handoff: {error}"))
    }
}

impl AuthorizedEffectExecutor for SystemdExecutorClient {
    fn execute_authorized(
        &mut self,
        authorization: AuthorizedEffect,
    ) -> Result<ExecutionOutcome, String> {
        let (effect, binding) = authorization.into_executor_request();
        let (unit, state) = parse_managed_effect(&effect)?;
        let proxy = zbus::blocking::Proxy::new(
            &self.connection,
            EXECUTOR_SERVICE,
            EXECUTOR_OBJECT,
            EXECUTOR_INTERFACE,
        )
        .map_err(|error| format!("cannot bind managed executor proxy: {error}"))?;

        let wire: ExecutorOutcomeWire = proxy
            .call(
                "SetManagedActiveState",
                &(
                    unit.as_str(),
                    state.as_str(),
                    binding.transaction_id.as_str(),
                    binding.generation,
                    binding.state_version,
                    binding.authority_binding_digest.to_hex(),
                    binding.authority_use_digest.to_hex(),
                    binding.effect_digest.to_hex(),
                    binding.dispatch_digest.to_hex(),
                ),
            )
            .map_err(|error| format!("managed executor transport failed: {error}"))?;

        let disposition = match wire.0.as_str() {
            "rejected-before-dispatch" => ExecutionDisposition::RejectedBeforeDispatch,
            "dispatched" => ExecutionDisposition::Dispatched,
            "indeterminate" => ExecutionDisposition::Indeterminate,
            value => return Err(format!("executor returned unknown disposition {value:?}")),
        };
        let dispatch_digest = if wire.1.is_empty()
            && disposition == ExecutionDisposition::RejectedBeforeDispatch
        {
            binding.dispatch_digest
        } else {
            ComponentDigest::parse_hex(&wire.1)
                .map_err(|error| format!("executor returned malformed dispatch digest: {error}"))?
        };
        if dispatch_digest != binding.dispatch_digest {
            return Err("executor response substituted the authorized dispatch digest".into());
        }
        ExecutionOutcome::new(disposition, dispatch_digest, wire.2)
            .map_err(|error| format!("executor returned invalid outcome: {error}"))
    }
}

struct FreshSystemdVerifier {
    observer: SystemdObserver,
    verifier: SystemdActiveStateVerifier,
}

impl Debug for FreshSystemdVerifier {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("FreshSystemdVerifier")
            .finish_non_exhaustive()
    }
}

impl FreshSystemdVerifier {
    fn connect() -> Result<Self, String> {
        Ok(Self {
            observer: SystemdObserver::connect()
                .map_err(|error| format!("cannot connect independent systemd observer: {error}"))?,
            verifier: SystemdActiveStateVerifier,
        })
    }
}

impl IndependentManagedVerifier for FreshSystemdVerifier {
    fn verify_effect(&mut self, effect: &EffectDescriptor) -> Result<VerificationOutcome, String> {
        let (unit, state) = parse_managed_effect(effect)?;
        let expectation = SystemdActiveStateExpectation::new(unit.as_str(), state.as_str())
            .map_err(|error| error.to_string())?;
        let capability =
            CapabilityId::new(MANAGED_SYSTEMD_CAPABILITY).map_err(|error| error.to_string())?;
        let observation = self
            .observer
            .observe_authoritative(&effect.resource, &capability)
            .map_err(|error| format!("independent systemd observation failed: {error}"))?;
        Ok(self.verifier.verify(&expectation, &observation))
    }
}

fn parse_managed_effect(
    effect: &EffectDescriptor,
) -> Result<(ManagedUnitName, ManagedActiveState), String> {
    if effect.provider.as_str() != MANAGED_SYSTEMD_PROVIDER
        || effect.operation != MANAGED_SYSTEMD_OPERATION
    {
        return Err("effect is outside the v0.6 managed systemd contract".into());
    }
    let payload = std::str::from_utf8(&effect.canonical_payload)
        .map_err(|_| "managed effect payload is not UTF-8".to_string())?;
    let mut lines = payload.lines();
    let unit = lines
        .next()
        .and_then(|line| line.strip_prefix("unit="))
        .ok_or_else(|| "managed effect is missing canonical unit".to_string())?;
    let state = lines
        .next()
        .and_then(|line| line.strip_prefix("active_state="))
        .ok_or_else(|| "managed effect is missing canonical active state".to_string())?;
    if lines.next().is_some() || !payload.ends_with('\n') {
        return Err("managed effect payload has trailing or non-canonical material".into());
    }
    let unit = ManagedUnitName::parse(unit).map_err(|error| error.to_string())?;
    let state = ManagedActiveState::parse(state).map_err(str::to_owned)?;
    let canonical = managed_active_state_effect(&unit, state).map_err(|error| error.to_string())?;
    if canonical != *effect {
        return Err("managed effect differs from the executor canonical encoding".into());
    }
    Ok((unit, state))
}

#[derive(Debug)]
struct AuthorityService {
    runtime: Arc<Mutex<ManagedRuntime>>,
}

impl AuthorityService {
    fn new(runtime: ManagedRuntime) -> Self {
        Self {
            runtime: Arc::new(Mutex::new(runtime)),
        }
    }

    async fn with_runtime<R, F>(&self, operation: F) -> zbus::fdo::Result<R>
    where
        R: Send + 'static,
        F: FnOnce(&mut ManagedRuntime) -> Result<R, String> + Send + 'static,
    {
        let runtime = Arc::clone(&self.runtime);
        blocking::unblock(move || {
            let mut guard = runtime
                .lock()
                .map_err(|_| "authority runtime lock is poisoned".to_string())?;
            operation(&mut guard)
        })
        .await
        .map_err(fdo_failed)
    }
}

#[zbus::interface(name = "org.linura.Authority1")]
impl AuthorityService {
    async fn converge_systemd_active_state(
        &self,
        operation_id: &str,
        unit: &str,
        desired_active_state: &str,
        reason: &str,
        #[zbus(connection)] connection: &zbus::Connection,
        #[zbus(header)] header: Header<'_>,
    ) -> zbus::fdo::Result<ReceiptWire> {
        let caller = authenticated_caller(connection, &header).await?;
        let sender = caller.unique_bus_name.clone();
        blocking::unblock(move || authorize_human_caller(&sender))
            .await
            .map_err(fdo_failed)?;

        let principal = AuthenticatedPrincipal::new(format!("unix:uid:{}", caller.uid))
            .map_err(|error| fdo_failed(error.to_string()))?;
        let principal_id = PrincipalId::new(principal.as_str().to_owned())
            .map_err(|error| fdo_failed(error.to_string()))?;
        let approval = TrustedHumanApproval::from_privileged_local_boundary(principal_id);
        let actor = Actor {
            id: ActorId::new(format!(
                "authority-dbus:v1:{}:{}:uid:{}:pid:{}",
                caller.unique_bus_name.len(),
                caller.unique_bus_name,
                caller.uid,
                caller.pid
            ))
            .map_err(|error| fdo_failed(error.to_string()))?,
            kind: ActorKind::Human,
            interactive: true,
        };
        let request = managed_request(operation_id, unit, desired_active_state, reason)
            .map_err(fdo_failed)?;

        self.with_runtime(move |runtime| {
            runtime
                .converge(principal, actor, request, &approval)
                .map(receipt_wire)
        })
        .await
    }
}

async fn authenticated_caller(
    connection: &zbus::Connection,
    header: &Header<'_>,
) -> zbus::fdo::Result<CallerCredentials> {
    let sender = header
        .sender()
        .ok_or_else(|| fdo_failed("authority request has no authenticated D-Bus sender"))?;
    let unique_bus_name = sender.as_str().to_owned();
    if !unique_bus_name.starts_with(':') || unique_bus_name.chars().any(char::is_control) {
        return Err(fdo_failed(
            "authority request sender is not a canonical unique bus name",
        ));
    }
    let proxy = zbus::Proxy::new(
        connection,
        "org.freedesktop.DBus",
        "/org/freedesktop/DBus",
        "org.freedesktop.DBus",
    )
    .await
    .map_err(|error| fdo_failed(error.to_string()))?;
    let uid: u32 = proxy
        .call("GetConnectionUnixUser", &(unique_bus_name.as_str(),))
        .await
        .map_err(|error| fdo_failed(error.to_string()))?;
    let pid: u32 = proxy
        .call("GetConnectionUnixProcessID", &(unique_bus_name.as_str(),))
        .await
        .map_err(|error| fdo_failed(error.to_string()))?;
    Ok(CallerCredentials {
        uid,
        pid,
        unique_bus_name,
    })
}

fn authorize_human_caller(sender: &str) -> Result<(), String> {
    let status = Command::new("/usr/bin/pkcheck")
        .args([
            "--action-id",
            HUMAN_ACTION_ID,
            "--system-bus-name",
            sender,
            "--allow-user-interaction",
        ])
        .status()
        .map_err(|error| format!("Polkit authorization service is unavailable: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err("administrator approval was denied or unavailable".into())
    }
}

fn managed_request(
    operation_id: &str,
    unit: &str,
    desired_active_state: &str,
    reason: &str,
) -> Result<PlanDesiredStateRequest, String> {
    if reason.is_empty() || reason.len() > MAX_REASON_BYTES || reason.chars().any(char::is_control)
    {
        return Err("reason must be 1..1024 bytes without control characters".into());
    }
    let unit = ManagedUnitName::parse(unit).map_err(|error| error.to_string())?;
    let state = ManagedActiveState::parse(desired_active_state).map_err(str::to_owned)?;
    let resource = unit.resource_id().map_err(|error| error.to_string())?;
    let mut request = PlanDesiredStateRequest {
        request_id: RequestId::new("request:v06:pending").map_err(|error| error.to_string())?,
        provider: ProviderId::new(MANAGED_SYSTEMD_PROVIDER).map_err(|error| error.to_string())?,
        resource,
        observation_capability: CapabilityId::new(MANAGED_SYSTEMD_CAPABILITY)
            .map_err(|error| error.to_string())?,
        reason: SemanticReason {
            summary: reason.to_owned(),
            intent_ids: vec![],
            requirement_ids: vec![],
            capability_ids: vec![],
        },
        desired_state: BTreeMap::from([("active_state".to_owned(), state.as_str().to_owned())]),
    };
    request.request_id =
        managed_request_id(operation_id, &request).map_err(|error| error.to_string())?;
    Ok(request)
}

fn receipt_wire(receipt: ManagedMutationReceipt) -> ReceiptWire {
    (
        receipt.transaction_id,
        receipt.plan_id,
        receipt.resource,
        receipt.desired_active_state,
        receipt.effect_digest,
        receipt.dispatch_digest.unwrap_or_default(),
        receipt.execution_disposition.unwrap_or_default(),
        receipt.verification_disposition,
        receipt.final_state,
        receipt.recovered,
        receipt.stages,
    )
}

fn prepare_state_dir(path: &Path) -> Result<(), String> {
    fs::create_dir_all(path)
        .map_err(|error| format!("cannot create authority state dir: {error}"))?;
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
        return Err(format!(
            "{} is not a non-zero 256-bit secret",
            path.display()
        ));
    }
    let metadata = fs::metadata(path).map_err(|error| error.to_string())?;
    if metadata.permissions().mode() & 0o077 != 0 {
        return Err(format!(
            "{} permissions are broader than 0600",
            path.display()
        ));
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

fn state_dir() -> PathBuf {
    env::var_os(STATE_DIR_ENV)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_STATE_DIR))
}

fn fdo_failed(message: impl Into<String>) -> zbus::fdo::Error {
    zbus::fdo::Error::Failed(message.into())
}

fn serve() -> Result<(), Box<dyn Error>> {
    let runtime = ManagedRuntime::open(&state_dir()).map_err(std::io::Error::other)?;
    let service = AuthorityService::new(runtime);
    let _connection = zbus::blocking::connection::Builder::system()?
        .name(SERVICE_NAME)?
        .serve_at(OBJECT_PATH, service)?
        .build()?;
    loop {
        std::thread::park();
    }
}

fn main() {
    if let Err(error) = serve() {
        eprintln!("linura-authorityd failed closed: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_request_surface_is_exactly_the_reserved_managed_effect() {
        let request = managed_request(
            "activate-test",
            "linura-managed-example.service",
            "active",
            "test convergence",
        )
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
            request
                .desired_state
                .get("active_state")
                .map(String::as_str),
            Some("active")
        );
        assert!(managed_request("bad", "sshd.service", "active", "test").is_err());
        assert!(
            managed_request("bad", "linura-managed-example.service", "failed", "test").is_err()
        );
    }

    #[test]
    fn canonical_effect_parser_rejects_extra_payload_material() {
        let unit = ManagedUnitName::parse("linura-managed-example.service")
            .unwrap_or_else(|error| unreachable!("{error}"));
        let effect = managed_active_state_effect(&unit, ManagedActiveState::Active)
            .unwrap_or_else(|error| unreachable!("{error}"));
        let parsed = parse_managed_effect(&effect).unwrap_or_else(|error| unreachable!("{error}"));
        assert_eq!(parsed.0, unit);
        assert_eq!(parsed.1, ManagedActiveState::Active);
    }
}

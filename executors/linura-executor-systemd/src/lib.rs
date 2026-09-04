#![forbid(unsafe_code)]

use std::fmt::{Display, Formatter};
use std::process::Command;

use linura_core::{ProviderId, ResourceId};
use linura_provider_sdk::{
    ComponentDigest, EffectDescriptor, ExecutionBinding, ExecutionDisposition, ExecutionOutcome,
};
use zbus::message::Header;
use zbus::zvariant::OwnedObjectPath;

pub const SERVICE_NAME: &str = "org.linura.Executor.Systemd1";
pub const OBJECT_PATH: &str = "/org/linura/Executor/Systemd1";
pub const INTERFACE_NAME: &str = "org.linura.Executor.Systemd1";
pub const QUALIFICATION_ACTION_ID: &str = "org.linura.executor.systemd.qualify-restart";
pub const QUALIFICATION_UNIT_PREFIX: &str = "linura-v05-qualification-";
const QUALIFICATION_OPERATION: &str = "restart-unit";
const MAX_WIRE_DETAIL_BYTES: usize = 192;

pub type QualificationOutcomeWire = (String, String, String);

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SystemdOperation {
    SetUnitEnabled { unit: UnitName, enabled: bool },
    RestartUnit { unit: UnitName },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnitName(String);

impl UnitName {
    pub fn parse(value: impl Into<String>) -> Result<Self, UnitNameError> {
        let value = value.into();
        if value.is_empty() || value.len() > 255 {
            return Err(UnitNameError::InvalidLength);
        }
        if !value.ends_with(".service") {
            return Err(UnitNameError::UnsupportedUnitType);
        }
        if !value.is_ascii()
            || value.chars().any(|character| {
                !(character.is_ascii_alphanumeric()
                    || matches!(character, ':' | '_' | '.' | '@' | '-'))
            })
        {
            return Err(UnitNameError::InvalidCharacter);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for UnitName {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnitNameError {
    InvalidLength,
    UnsupportedUnitType,
    InvalidCharacter,
}

impl Display for UnitNameError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidLength => f.write_str("systemd unit name has invalid length"),
            Self::UnsupportedUnitType => f.write_str("only systemd .service units are accepted"),
            Self::InvalidCharacter => {
                f.write_str("systemd unit name contains an invalid character")
            }
        }
    }
}

impl std::error::Error for UnitNameError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QualificationUnitName(UnitName);

impl QualificationUnitName {
    pub fn parse(value: impl Into<String>) -> Result<Self, QualificationUnitError> {
        let unit = UnitName::parse(value).map_err(QualificationUnitError::Unit)?;
        let Some(suffix) = unit.as_str().strip_prefix(QUALIFICATION_UNIT_PREFIX) else {
            return Err(QualificationUnitError::WrongNamespace);
        };
        let Some(slug) = suffix.strip_suffix(".service") else {
            return Err(QualificationUnitError::WrongNamespace);
        };
        if slug.is_empty()
            || slug.len() > 96
            || !slug.chars().all(|character| {
                character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
            })
            || slug.starts_with('-')
            || slug.ends_with('-')
            || slug.contains("--")
        {
            return Err(QualificationUnitError::InvalidFixtureName);
        }
        Ok(Self(unit))
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    pub fn resource_id(&self) -> Result<ResourceId, ExecutorError> {
        ResourceId::new(format!("systemd:unit:{}", self.as_str()))
            .map_err(|error| ExecutorError::Contract(error.to_string()))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum QualificationUnitError {
    Unit(UnitNameError),
    WrongNamespace,
    InvalidFixtureName,
}

impl Display for QualificationUnitError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unit(error) => write!(f, "{error}"),
            Self::WrongNamespace => f.write_str("unit is outside the v0.5 qualification namespace"),
            Self::InvalidFixtureName => f.write_str("qualification fixture name is not canonical"),
        }
    }
}

impl std::error::Error for QualificationUnitError {}

#[derive(Debug)]
pub enum ExecutorError {
    Contract(String),
    Transport(String),
}

impl Display for ExecutorError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Contract(detail) => write!(f, "executor contract error: {detail}"),
            Self::Transport(detail) => write!(f, "executor transport error: {detail}"),
        }
    }
}

impl std::error::Error for ExecutorError {}

#[derive(Clone, Debug, Default)]
pub struct SystemdExecutorService;

#[zbus::interface(name = "org.linura.Executor.Systemd1")]
impl SystemdExecutorService {
    /// Qualification-only v0.5 fixture restart.
    ///
    /// This is deliberately not a product mutation surface. The supplied
    /// binding is exact correlation material, not bearer authority. The caller
    /// must separately be authenticated by the system bus and authorized by
    /// the dedicated Polkit action.
    // The qualification-only D-Bus ABI deliberately carries each exact binding field
    // separately, plus zbus-injected connection/header context. Collapsing these fields
    // would weaken wire-level reviewability solely to satisfy a local style heuristic.
    #[allow(clippy::too_many_arguments)]
    async fn qualify_restart(
        &self,
        unit: &str,
        transaction_id: &str,
        generation: u64,
        state_version: u64,
        authority_binding_digest: &str,
        authority_use_digest: &str,
        effect_digest: &str,
        dispatch_digest: &str,
        #[zbus(connection)] connection: &zbus::Connection,
        #[zbus(header)] header: Header<'_>,
    ) -> zbus::fdo::Result<QualificationOutcomeWire> {
        let sender = authenticated_sender(&header)?;
        authorize_qualification_caller(&sender)?;

        let unit = match QualificationUnitName::parse(unit) {
            Ok(unit) => unit,
            Err(error) => return Ok(rejected_wire(error.to_string())),
        };
        let effect = match restart_effect(&unit) {
            Ok(effect) => effect,
            Err(error) => return Ok(rejected_wire(error.to_string())),
        };
        let binding = match binding_from_wire(
            transaction_id,
            generation,
            state_version,
            authority_binding_digest,
            authority_use_digest,
            effect_digest,
            dispatch_digest,
        ) {
            Ok(binding) => binding,
            Err(error) => return Ok(rejected_wire(error)),
        };
        if let Err(error) = binding.validate_for(&effect) {
            return Ok(rejected_wire(error.to_string()));
        }

        let proxy = match zbus::Proxy::new(
            connection,
            "org.freedesktop.systemd1",
            "/org/freedesktop/systemd1",
            "org.freedesktop.systemd1.Manager",
        )
        .await
        {
            Ok(proxy) => proxy,
            Err(error) => {
                return Ok(outcome_wire(bounded_outcome(
                    ExecutionDisposition::RejectedBeforeDispatch,
                    binding.dispatch_digest,
                    &format!(
                        "systemd proxy unavailable: {}",
                        bounded_text(&error.to_string())
                    ),
                )?));
            }
        };

        let dispatch: Result<OwnedObjectPath, zbus::Error> =
            proxy.call("RestartUnit", &(unit.as_str(), "replace")).await;
        match dispatch {
            Ok(_job) => Ok(outcome_wire(bounded_outcome(
                ExecutionDisposition::Dispatched,
                binding.dispatch_digest,
                "systemd RestartUnit accepted; authoritative verification required",
            )?)),
            Err(error) => Ok(outcome_wire(bounded_outcome(
                ExecutionDisposition::Indeterminate,
                binding.dispatch_digest,
                &format!(
                    "systemd dispatch outcome is indeterminate: {}",
                    bounded_text(&error.to_string())
                ),
            )?)),
        }
    }
}

pub fn serve() -> Result<(), ExecutorError> {
    let _connection = zbus::blocking::connection::Builder::system()
        .map_err(|error| ExecutorError::Transport(error.to_string()))?
        .name(SERVICE_NAME)
        .map_err(|error| ExecutorError::Transport(error.to_string()))?
        .serve_at(OBJECT_PATH, SystemdExecutorService)
        .map_err(|error| ExecutorError::Transport(error.to_string()))?
        .build()
        .map_err(|error| ExecutorError::Transport(error.to_string()))?;
    loop {
        std::thread::park();
    }
}

pub fn restart_effect(unit: &QualificationUnitName) -> Result<EffectDescriptor, ExecutorError> {
    let provider =
        ProviderId::new("systemd").map_err(|error| ExecutorError::Contract(error.to_string()))?;
    let resource = unit.resource_id()?;
    EffectDescriptor::new(
        provider,
        resource,
        QUALIFICATION_OPERATION,
        unit.as_str().as_bytes().to_vec(),
    )
    .map_err(|error| ExecutorError::Contract(error.to_string()))
}

fn binding_from_wire(
    transaction_id: &str,
    generation: u64,
    state_version: u64,
    authority_binding_digest: &str,
    authority_use_digest: &str,
    effect_digest: &str,
    dispatch_digest: &str,
) -> Result<ExecutionBinding, String> {
    Ok(ExecutionBinding {
        transaction_id: transaction_id.into(),
        generation,
        state_version,
        authority_binding_digest: ComponentDigest::parse_hex(authority_binding_digest)
            .map_err(|error| error.to_string())?,
        authority_use_digest: ComponentDigest::parse_hex(authority_use_digest)
            .map_err(|error| error.to_string())?,
        effect_digest: ComponentDigest::parse_hex(effect_digest)
            .map_err(|error| error.to_string())?,
        dispatch_digest: ComponentDigest::parse_hex(dispatch_digest)
            .map_err(|error| error.to_string())?,
    })
}

fn authenticated_sender(header: &Header<'_>) -> zbus::fdo::Result<String> {
    let sender = header.sender().ok_or_else(|| {
        zbus::fdo::Error::AccessDenied("method call has no authenticated D-Bus sender".into())
    })?;
    let sender = sender.as_str();
    if sender.is_empty()
        || sender.len() > 255
        || !sender.starts_with(':')
        || sender.chars().any(char::is_control)
    {
        return Err(zbus::fdo::Error::AccessDenied(
            "method call sender is not a canonical unique bus name".into(),
        ));
    }
    Ok(sender.into())
}

fn authorize_qualification_caller(sender: &str) -> zbus::fdo::Result<()> {
    let status = Command::new("/usr/bin/pkcheck")
        .args([
            "--action-id",
            QUALIFICATION_ACTION_ID,
            "--system-bus-name",
            sender,
        ])
        .status()
        .map_err(|_| {
            zbus::fdo::Error::AccessDenied(
                "qualification authorization service is unavailable".into(),
            )
        })?;
    if status.success() {
        Ok(())
    } else {
        Err(zbus::fdo::Error::AccessDenied(
            "caller is not authorized for v0.5 executor qualification".into(),
        ))
    }
}

fn rejected_wire(detail: String) -> QualificationOutcomeWire {
    (
        "rejected-before-dispatch".into(),
        String::new(),
        bounded_text(&detail),
    )
}

fn bounded_outcome(
    disposition: ExecutionDisposition,
    dispatch_digest: ComponentDigest,
    detail: &str,
) -> zbus::fdo::Result<ExecutionOutcome> {
    ExecutionOutcome::new(disposition, dispatch_digest, bounded_text(detail))
        .map_err(|error| zbus::fdo::Error::Failed(error.to_string()))
}

fn outcome_wire(outcome: ExecutionOutcome) -> QualificationOutcomeWire {
    (
        match outcome.disposition {
            ExecutionDisposition::RejectedBeforeDispatch => "rejected-before-dispatch",
            ExecutionDisposition::Dispatched => "dispatched",
            ExecutionDisposition::Indeterminate => "indeterminate",
        }
        .into(),
        outcome.dispatch_digest.to_hex(),
        outcome.detail,
    )
}

fn bounded_text(value: &str) -> String {
    let mut output = String::with_capacity(value.len().min(MAX_WIRE_DETAIL_BYTES));
    for character in value.chars() {
        if character.is_control() {
            output.push(' ');
        } else {
            output.push(character);
        }
        if output.len() >= MAX_WIRE_DETAIL_BYTES {
            break;
        }
    }
    while !output.is_char_boundary(output.len()) {
        output.pop();
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use linura_provider_sdk::ExecutionBinding;

    fn id<T, E: std::fmt::Debug>(value: Result<T, E>) -> T {
        match value {
            Ok(value) => value,
            Err(error) => unreachable!("{error:?}"),
        }
    }

    fn digest(byte: u8) -> ComponentDigest {
        ComponentDigest::from_bytes([byte; 32])
    }

    #[test]
    fn only_service_units_are_accepted_initially() {
        assert!(UnitName::parse("sshd.service").is_ok());
        assert_eq!(
            UnitName::parse("multi-user.target"),
            Err(UnitNameError::UnsupportedUnitType)
        );
    }

    #[test]
    fn suspicious_unit_names_are_rejected() {
        for value in [
            "../../evil.service",
            "bad name.service",
            "bad;name.service",
            "bad$name.service",
            "bad\nname.service",
            "bad\\name.service",
        ] {
            assert!(
                UnitName::parse(value).is_err(),
                "accepted hostile unit: {value}"
            );
        }
    }

    #[test]
    fn qualification_namespace_is_exact_and_canonical() {
        assert!(QualificationUnitName::parse("linura-v05-qualification-restart.service").is_ok());
        for value in [
            "sshd.service",
            "linura-v05-qualification-.service",
            "linura-v05-qualification--bad.service",
            "linura-v05-qualification-Bad.service",
            "linura-v05-qualification-bad--slug.service",
        ] {
            assert!(
                QualificationUnitName::parse(value).is_err(),
                "accepted fixture: {value}"
            );
        }
    }

    #[test]
    fn exact_binding_rejects_effect_substitution() {
        let first = id(QualificationUnitName::parse(
            "linura-v05-qualification-first.service",
        ));
        let second = id(QualificationUnitName::parse(
            "linura-v05-qualification-second.service",
        ));
        let first_effect = id(restart_effect(&first));
        let second_effect = id(restart_effect(&second));
        let binding = id(ExecutionBinding::new(
            "tx:v05-test",
            1,
            1,
            digest(1),
            digest(2),
            &first_effect,
        ));
        assert!(binding.validate_for(&first_effect).is_ok());
        assert!(binding.validate_for(&second_effect).is_err());
    }

    #[test]
    fn wire_binding_rejects_malformed_digests() {
        assert!(
            binding_from_wire(
                "tx",
                1,
                1,
                "bad",
                &"2".repeat(64),
                &"3".repeat(64),
                &"4".repeat(64)
            )
            .is_err()
        );
    }

    #[test]
    fn error_text_is_control_free_and_bounded() {
        let bounded = bounded_text(&format!("bad\n{}", "x".repeat(500)));
        assert!(!bounded.chars().any(char::is_control));
        assert!(bounded.len() <= MAX_WIRE_DETAIL_BYTES);
    }
}

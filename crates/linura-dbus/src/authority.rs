use std::fmt::{Debug, Formatter};
use std::process::Command;
use std::sync::{Arc, Mutex};

use linura_control::{
    AuthenticatedPrincipal, ManagedMutationReceipt, TrustedHumanApproval,
};
use linura_core::{Actor, ActorId, ActorKind, PrincipalId};
use zbus::message::Header;

use super::{
    ContractAnnotatedInterface, TransportError, authenticated_caller, fdo_failed,
    principal_from_caller,
};

pub const AUTHORITY_SERVICE_NAME: &str = "org.linura.Authority1";
pub const AUTHORITY_OBJECT_PATH: &str = "/org/linura/Authority1";
pub const AUTHORITY_INTERFACE_NAME: &str = "org.linura.Authority1";
pub const AUTHORITY_CONTRACT_ID: &str = "dbus.org.linura.Authority1";
pub const AUTHORITY_CONTRACT_VERSION: &str = "1";
pub const AUTHORITY_CONTRACT_STABILITY: &str = "experimental";
pub const MANAGE_SYSTEMD_ACTIVE_STATE_ACTION: &str =
    "org.linura.authority.manage-systemd-active-state";

const AUTHORITY_CONTRACT_ANNOTATIONS: [(&str, &str); 3] = [
    ("org.linura.ContractId", AUTHORITY_CONTRACT_ID),
    ("org.linura.ContractVersion", AUTHORITY_CONTRACT_VERSION),
    ("org.linura.Stability", AUTHORITY_CONTRACT_STABILITY),
];

pub type AuthorityReceiptWire = (
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

pub struct Authority1Context {
    pub principal: AuthenticatedPrincipal,
    pub actor: Actor,
    pub approval: TrustedHumanApproval,
}

impl Debug for Authority1Context {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Authority1Context")
            .field("principal", &self.principal.as_str())
            .field("actor", &self.actor)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Authority1ManagedRequest {
    pub operation_id: String,
    pub unit: String,
    pub desired_active_state: String,
    pub reason: String,
}

pub trait Authority1Handler: Send + 'static {
    fn converge_systemd_active_state(
        &mut self,
        context: Authority1Context,
        request: Authority1ManagedRequest,
    ) -> Result<ManagedMutationReceipt, String>;
}

struct Authority1Service {
    handler: Arc<Mutex<Box<dyn Authority1Handler>>>,
}

impl Debug for Authority1Service {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Authority1Service")
            .finish_non_exhaustive()
    }
}

impl Authority1Service {
    fn new(handler: impl Authority1Handler) -> Self {
        Self {
            handler: Arc::new(Mutex::new(Box::new(handler))),
        }
    }

    async fn with_handler<R, F>(&self, operation: F) -> zbus::fdo::Result<R>
    where
        R: Send + 'static,
        F: FnOnce(&mut dyn Authority1Handler) -> Result<R, String> + Send + 'static,
    {
        let handler = Arc::clone(&self.handler);
        blocking::unblock(move || {
            let mut guard = handler
                .lock()
                .map_err(|_| "Authority1 handler lock is poisoned".to_owned())?;
            operation(guard.as_mut())
        })
        .await
        .map_err(fdo_failed)
    }
}

#[zbus::interface(name = "org.linura.Authority1")]
impl Authority1Service {
    async fn converge_systemd_active_state(
        &self,
        operation_id: &str,
        unit: &str,
        desired_active_state: &str,
        reason: &str,
        #[zbus(connection)] connection: &zbus::Connection,
        #[zbus(header)] header: Header<'_>,
    ) -> zbus::fdo::Result<AuthorityReceiptWire> {
        let caller = authenticated_caller(connection, &header).await?;
        let sender = caller.unique_bus_name.clone();
        blocking::unblock(move || authorize_human_caller(&sender))
            .await
            .map_err(fdo_failed)?;

        let principal = principal_from_caller(&caller)?;
        let principal_id = PrincipalId::new(principal.as_str().to_owned())
            .map_err(|error| fdo_failed(error.to_string()))?;
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
        let context = Authority1Context {
            principal,
            actor,
            approval: TrustedHumanApproval::from_privileged_local_boundary(principal_id),
        };
        let request = Authority1ManagedRequest {
            operation_id: operation_id.to_owned(),
            unit: unit.to_owned(),
            desired_active_state: desired_active_state.to_owned(),
            reason: reason.to_owned(),
        };

        self.with_handler(move |handler| {
            handler
                .converge_systemd_active_state(context, request)
                .map(receipt_wire)
        })
        .await
    }
}

fn authorize_human_caller(sender: &str) -> Result<(), String> {
    let status = Command::new("/usr/bin/pkcheck")
        .args([
            "--action-id",
            MANAGE_SYSTEMD_ACTIVE_STATE_ACTION,
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

fn receipt_wire(receipt: ManagedMutationReceipt) -> AuthorityReceiptWire {
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

fn authority1_service(
    handler: impl Authority1Handler,
) -> ContractAnnotatedInterface<Authority1Service> {
    ContractAnnotatedInterface::new(
        Authority1Service::new(handler),
        &AUTHORITY_CONTRACT_ANNOTATIONS,
    )
}

pub fn serve_authority1(handler: impl Authority1Handler) -> Result<(), TransportError> {
    let service = authority1_service(handler);
    let _connection = zbus::blocking::connection::Builder::system()?
        .name(AUTHORITY_SERVICE_NAME)?
        .serve_at(AUTHORITY_OBJECT_PATH, service)?
        .build()?;
    loop {
        std::thread::park();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zbus::object_server::Interface;

    #[derive(Debug)]
    struct NeverCalled;

    impl Authority1Handler for NeverCalled {
        fn converge_systemd_active_state(
            &mut self,
            _context: Authority1Context,
            _request: Authority1ManagedRequest,
        ) -> Result<ManagedMutationReceipt, String> {
            Err("introspection must not call the authority handler".into())
        }
    }

    #[test]
    fn authority_contract_is_explicitly_experimental() {
        assert_eq!(AUTHORITY_SERVICE_NAME, "org.linura.Authority1");
        assert_eq!(AUTHORITY_INTERFACE_NAME, "org.linura.Authority1");
        assert_eq!(AUTHORITY_CONTRACT_ID, "dbus.org.linura.Authority1");
        assert_eq!(AUTHORITY_CONTRACT_VERSION, "1");
        assert_eq!(AUTHORITY_CONTRACT_STABILITY, "experimental");
    }

    #[test]
    fn live_authority_introspection_matches_canonical_surface() {
        let service = authority1_service(NeverCalled);
        let mut live = String::new();
        service.introspect_to_writer(&mut live, 0);
        let canonical = include_str!("../../../interfaces/dbus/org.linura.Authority1.xml");

        for &(name, value) in &AUTHORITY_CONTRACT_ANNOTATIONS {
            let marker = format!("name=\"{name}\" value=\"{value}\"");
            assert_eq!(canonical.matches(&marker).count(), 1, "canonical {name}");
            assert_eq!(live.matches(&marker).count(), 1, "live {name}");
        }
        let method = "<method name=\"ConvergeSystemdActiveState\">";
        assert!(canonical.contains(method));
        assert!(live.contains(method));
        for argument in [
            "operation_id",
            "unit",
            "desired_active_state",
            "reason",
        ] {
            let marker = format!("name=\"{argument}\" type=\"s\" direction=\"in\"");
            assert!(canonical.contains(&marker), "canonical {argument}");
            assert!(live.contains(&marker), "live {argument}");
        }
        let receipt = "type=\"(sssssssssbas)\" direction=\"out\"";
        assert_eq!(canonical.matches(receipt).count(), 1, "canonical receipt");
        assert_eq!(live.matches(receipt).count(), 1, "live receipt");
    }
}

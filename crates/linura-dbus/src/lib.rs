#![forbid(unsafe_code)]

use std::fmt::{Debug, Display, Formatter};
use std::sync::{Arc, Mutex};

use linura_core::{Actor, ActorId, ActorKind, CapabilityId, ProviderId, ResourceId, SupportLevel};
use linura_graph::{EdgeKind, NodeId, SystemGraph};
use linura_observation_control::{ObservationControlError, ObservationCoordinator};
use linura_protocol::{
    ObservationExplanation, ObservationRequest, ObservationResponse, ProviderSnapshot,
};
use zbus::blocking::{Connection as BlockingConnection, Proxy as BlockingProxy};
use zbus::message::Header;

pub const SERVICE_NAME: &str = "org.linura.Control";
pub const OBJECT_PATH: &str = "/org/linura/Control1";
pub const INTERFACE_NAME: &str = "org.linura.Control1";

pub type CallerWire = (String, String, bool, u32, u32, String);
pub type ProviderWire = (String, String, String);
pub type CapabilityWire = (String, String, String, String);
pub type CapabilitiesWire = (Vec<ProviderWire>, Vec<CapabilityWire>);
pub type ObservationWire = (
    String,
    String,
    String,
    String,
    String,
    u64,
    u64,
    u64,
    Vec<(String, String)>,
);
pub type GraphWire = (
    Vec<(String, Vec<(String, String)>)>,
    Vec<(String, String, String, String)>,
);
pub type ExplanationWire = (String, String, String, String, String, String);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CallerIdentity {
    pub actor: Actor,
    pub uid: u32,
    pub pid: u32,
    pub unique_bus_name: String,
}

impl CallerIdentity {
    #[must_use]
    pub fn to_wire(&self) -> CallerWire {
        (
            self.actor.id.as_str().into(),
            actor_kind_name(self.actor.kind).into(),
            self.actor.interactive,
            self.uid,
            self.pid,
            self.unique_bus_name.clone(),
        )
    }
}

#[derive(Debug)]
pub struct Control1Service {
    coordinator: Arc<Mutex<ObservationCoordinator>>,
}

impl Control1Service {
    #[must_use]
    pub fn new(coordinator: ObservationCoordinator) -> Self {
        Self {
            coordinator: Arc::new(Mutex::new(coordinator)),
        }
    }

    async fn with_coordinator<R, F>(&self, operation: F) -> zbus::fdo::Result<R>
    where
        R: Send + 'static,
        F: FnOnce(&mut ObservationCoordinator) -> Result<R, ObservationControlError>
            + Send
            + 'static,
    {
        let coordinator = Arc::clone(&self.coordinator);
        blocking::unblock(move || {
            let mut guard = coordinator
                .lock()
                .map_err(|_| "observation coordinator lock is poisoned".to_string())?;
            operation(&mut guard).map_err(|error| error.to_string())
        })
        .await
        .map_err(fdo_failed)
    }
}

#[zbus::interface(name = "org.linura.Control1")]
impl Control1Service {
    async fn who_am_i(
        &self,
        #[zbus(connection)] connection: &zbus::Connection,
        #[zbus(header)] header: Header<'_>,
    ) -> zbus::fdo::Result<CallerWire> {
        authenticated_caller(connection, &header)
            .await
            .map(|caller| caller.to_wire())
    }

    async fn capabilities(
        &self,
        #[zbus(connection)] connection: &zbus::Connection,
        #[zbus(header)] header: Header<'_>,
    ) -> zbus::fdo::Result<CapabilitiesWire> {
        let _caller = authenticated_caller(connection, &header).await?;
        self.with_coordinator(|coordinator| coordinator.provider_snapshot().map(provider_wire))
            .await
    }

    async fn observe(
        &self,
        provider: &str,
        resource: &str,
        capability: &str,
        #[zbus(connection)] connection: &zbus::Connection,
        #[zbus(header)] header: Header<'_>,
    ) -> zbus::fdo::Result<ObservationWire> {
        let _caller = authenticated_caller(connection, &header).await?;
        let request = ObservationRequest {
            provider: parse_provider(provider)?,
            resource: parse_resource(resource)?,
            capability: parse_capability(capability)?,
        };
        let now = unix_time_ms()?;
        self.with_coordinator(move |coordinator| {
            coordinator.observe(&request, now).map(observation_wire)
        })
        .await
    }

    async fn graph(
        &self,
        #[zbus(connection)] connection: &zbus::Connection,
        #[zbus(header)] header: Header<'_>,
    ) -> zbus::fdo::Result<GraphWire> {
        let _caller = authenticated_caller(connection, &header).await?;
        self.with_coordinator(|coordinator| Ok(graph_wire(coordinator.graph())))
            .await
    }

    async fn explain(
        &self,
        resource: &str,
        #[zbus(connection)] connection: &zbus::Connection,
        #[zbus(header)] header: Header<'_>,
    ) -> zbus::fdo::Result<ExplanationWire> {
        let _caller = authenticated_caller(connection, &header).await?;
        let resource = parse_resource(resource)?;
        let now = unix_time_ms()?;
        self.with_coordinator(move |coordinator| {
            coordinator.explain(&resource, now).map(explanation_wire)
        })
        .await
    }
}

pub fn serve(coordinator: ObservationCoordinator) -> Result<(), TransportError> {
    let service = Control1Service::new(coordinator);
    let _connection = zbus::blocking::connection::Builder::session()?
        .name(SERVICE_NAME)?
        .serve_at(OBJECT_PATH, service)?
        .build()?;
    loop {
        std::thread::park();
    }
}

#[derive(Debug)]
pub struct Control1Client {
    connection: BlockingConnection,
}

impl Control1Client {
    pub fn connect() -> Result<Self, TransportError> {
        Ok(Self {
            connection: BlockingConnection::session()?,
        })
    }

    fn proxy(&self) -> Result<BlockingProxy<'_>, TransportError> {
        BlockingProxy::new(&self.connection, SERVICE_NAME, OBJECT_PATH, INTERFACE_NAME)
            .map_err(TransportError::from)
    }

    pub fn who_am_i(&self) -> Result<CallerWire, TransportError> {
        self.proxy()?
            .call("WhoAmI", &())
            .map_err(TransportError::from)
    }

    pub fn capabilities(&self) -> Result<CapabilitiesWire, TransportError> {
        self.proxy()?
            .call("Capabilities", &())
            .map_err(TransportError::from)
    }

    pub fn observe(
        &self,
        provider: &str,
        resource: &str,
        capability: &str,
    ) -> Result<ObservationWire, TransportError> {
        self.proxy()?
            .call("Observe", &(provider, resource, capability))
            .map_err(TransportError::from)
    }

    pub fn graph(&self) -> Result<GraphWire, TransportError> {
        self.proxy()?
            .call("Graph", &())
            .map_err(TransportError::from)
    }

    pub fn explain(&self, resource: &str) -> Result<ExplanationWire, TransportError> {
        self.proxy()?
            .call("Explain", &(resource,))
            .map_err(TransportError::from)
    }
}

#[derive(Debug)]
pub struct TransportError {
    message: String,
}

impl TransportError {
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl Display for TransportError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for TransportError {}

impl From<zbus::Error> for TransportError {
    fn from(error: zbus::Error) -> Self {
        Self::new(error.to_string())
    }
}

async fn authenticated_caller(
    connection: &zbus::Connection,
    header: &Header<'_>,
) -> zbus::fdo::Result<CallerIdentity> {
    let sender = header
        .sender()
        .ok_or_else(|| fdo_failed("D-Bus method call has no authenticated sender"))?;
    let unique_bus_name = sender.as_str().to_string();
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
    actor_from_credentials(&unique_bus_name, uid, pid)
}

fn actor_from_credentials(
    unique_bus_name: &str,
    uid: u32,
    pid: u32,
) -> zbus::fdo::Result<CallerIdentity> {
    let actor_id = ActorId::new(format!(
        "dbus:v1:{}:{unique_bus_name}:uid:{uid}:pid:{pid}",
        unique_bus_name.len()
    ))
    .map_err(|error| fdo_failed(error.to_string()))?;
    Ok(CallerIdentity {
        actor: Actor {
            id: actor_id,
            kind: ActorKind::Service,
            interactive: false,
        },
        uid,
        pid,
        unique_bus_name: unique_bus_name.into(),
    })
}

fn provider_wire(snapshot: ProviderSnapshot) -> CapabilitiesWire {
    let providers = snapshot
        .providers
        .into_iter()
        .map(|health| {
            (
                health.provider.as_str().into(),
                health.availability.as_str().into(),
                health.reason.unwrap_or_default(),
            )
        })
        .collect();
    let capabilities = snapshot
        .capabilities
        .into_iter()
        .map(|capability| {
            (
                capability.id.as_str().into(),
                capability
                    .provider
                    .as_ref()
                    .map_or_else(String::new, |provider| provider.as_str().into()),
                support_level_name(capability.support).into(),
                capability.reason.unwrap_or_default(),
            )
        })
        .collect();
    (providers, capabilities)
}

fn observation_wire(response: ObservationResponse) -> ObservationWire {
    let observation = response.observation;
    let attributes = observation
        .attributes
        .into_iter()
        .map(|(key, value)| (key, value.to_string()))
        .collect();
    (
        observation.provider.as_str().into(),
        observation.resource.as_str().into(),
        observation.capability.as_str().into(),
        observation.authority.as_str().into(),
        response.freshness.as_str().into(),
        observation.observed_at_unix_ms,
        observation.valid_for_ms,
        observation.sequence,
        attributes,
    )
}

fn graph_wire(graph: &SystemGraph) -> GraphWire {
    let nodes = graph
        .nodes()
        .map(|node| {
            (
                node_id_name(&node.id),
                node.attributes
                    .iter()
                    .map(|(key, value)| (key.clone(), value.clone()))
                    .collect(),
            )
        })
        .collect();
    let mut edges: Vec<_> = graph
        .edges()
        .iter()
        .map(|edge| {
            (
                node_id_name(&edge.from),
                node_id_name(&edge.to),
                edge_kind_name(edge.kind).into(),
                edge.reason.clone(),
            )
        })
        .collect();
    edges.sort();
    (nodes, edges)
}

fn explanation_wire(explanation: ObservationExplanation) -> ExplanationWire {
    (
        explanation.resource.as_str().into(),
        explanation.provider.as_str().into(),
        explanation.capability.as_str().into(),
        explanation.freshness.as_str().into(),
        explanation.evidence_id,
        explanation.authority,
    )
}

fn node_id_name(node: &NodeId) -> String {
    match node {
        NodeId::Intent(id) => format!("intent:{}", id.as_str()),
        NodeId::Setup(id) => format!("setup:{}", id.as_str()),
        NodeId::Requirement(id) => format!("requirement:{}", id.as_str()),
        NodeId::Capability(id) => format!("capability:{}", id.as_str()),
        NodeId::Provider(id) => format!("provider:{}", id.as_str()),
        NodeId::Resource(id) => format!("resource:{}", id.as_str()),
        NodeId::Evidence(id) => format!("evidence:{id}"),
        NodeId::Workflow(id) => format!("workflow:{id}"),
    }
}

const fn edge_kind_name(kind: EdgeKind) -> &'static str {
    match kind {
        EdgeKind::Requires => "requires",
        EdgeKind::Provides => "provides",
        EdgeKind::Conflicts => "conflicts",
        EdgeKind::Replaces => "replaces",
        EdgeKind::Recommends => "recommends",
        EdgeKind::Optional => "optional",
        EdgeKind::Owns => "owns",
        EdgeKind::SharedBy => "shared-by",
        EdgeKind::DerivedFrom => "derived-from",
        EdgeKind::Realizes => "realizes",
        EdgeKind::ObservedBy => "observed-by",
        EdgeKind::EvidenceFor => "evidence-for",
    }
}

const fn actor_kind_name(kind: ActorKind) -> &'static str {
    match kind {
        ActorKind::Human => "human",
        ActorKind::Service => "service",
        ActorKind::Agent => "agent",
        ActorKind::Remote => "remote",
    }
}

const fn support_level_name(level: SupportLevel) -> &'static str {
    match level {
        SupportLevel::Supported => "supported",
        SupportLevel::Unsupported => "unsupported",
        SupportLevel::Degraded => "degraded",
        SupportLevel::Unknown => "unknown",
    }
}

fn parse_provider(value: &str) -> zbus::fdo::Result<ProviderId> {
    ProviderId::new(value).map_err(|error| fdo_failed(error.to_string()))
}

fn parse_resource(value: &str) -> zbus::fdo::Result<ResourceId> {
    ResourceId::new(value).map_err(|error| fdo_failed(error.to_string()))
}

fn parse_capability(value: &str) -> zbus::fdo::Result<CapabilityId> {
    CapabilityId::new(value).map_err(|error| fdo_failed(error.to_string()))
}

fn unix_time_ms() -> zbus::fdo::Result<u64> {
    let duration = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|error| fdo_failed(error.to_string()))?;
    u64::try_from(duration.as_millis())
        .map_err(|_| fdo_failed("Unix timestamp exceeds u64 milliseconds"))
}

fn fdo_failed(message: impl Into<String>) -> zbus::fdo::Error {
    zbus::fdo::Error::Failed(message.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn actor_identity_binds_bus_name_uid_and_pid() {
        let first = actor_from_credentials(":1.42", 1000, 2000)
            .unwrap_or_else(|error| unreachable!("{error}"));
        let second = actor_from_credentials(":1.43", 1000, 2000)
            .unwrap_or_else(|error| unreachable!("{error}"));
        assert_ne!(first.actor.id, second.actor.id);
        assert_eq!(first.uid, 1000);
        assert_eq!(first.pid, 2000);
        assert!(!first.actor.interactive);
        assert_eq!(first.actor.kind, ActorKind::Service);
    }

    #[test]
    fn actor_identity_rejects_malformed_bus_identity() {
        assert!(actor_from_credentials(":1.42\nspoof", 1000, 2000).is_err());
    }

    #[test]
    fn node_wire_ids_are_type_namespaced() {
        let provider = ProviderId::new("systemd").unwrap_or_else(|error| unreachable!("{error}"));
        assert_eq!(
            node_id_name(&NodeId::Provider(provider)),
            "provider:systemd"
        );
    }
}

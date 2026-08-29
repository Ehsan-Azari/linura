#![forbid(unsafe_code)]

use std::fmt::{Debug, Formatter};

use linura_graph::SystemGraph;
use linura_intent::Intent;
use linura_policy::{PolicyDecision, PolicyEngine};
use linura_protocol::{ActionRequest, PlanResponse};
use linura_provider_sdk::{Provider, ProviderError};

/// Linura's local authority/control plane.
///
/// This component is deliberately independent of model providers. Agents and
/// other clients may request or propose work, but only this authority boundary
/// may turn validated requests into policy-evaluated plans.
pub struct ControlPlane<P> {
    policy: P,
    providers: Vec<Box<dyn Provider>>,
    graph: SystemGraph,
    intents: Vec<Intent>,
}

impl<P: Debug> Debug for ControlPlane<P> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ControlPlane")
            .field("policy", &self.policy)
            .field("provider_count", &self.providers.len())
            .field("intent_count", &self.intents.len())
            .field("graph_edge_count", &self.graph.edges().len())
            .finish()
    }
}

impl<P: PolicyEngine> ControlPlane<P> {
    #[must_use]
    pub fn new(policy: P) -> Self {
        Self {
            policy,
            providers: Vec::new(),
            graph: SystemGraph::default(),
            intents: Vec::new(),
        }
    }

    pub fn register_provider(&mut self, provider: Box<dyn Provider>) {
        self.providers.push(provider);
    }

    #[must_use]
    pub fn graph(&self) -> &SystemGraph {
        &self.graph
    }

    #[must_use]
    pub fn intents(&self) -> &[Intent] {
        &self.intents
    }

    pub fn plan(
        &self,
        request: &ActionRequest,
    ) -> Result<(PlanResponse, Option<PolicyDecision>), ProviderError> {
        let Some(provider) = self
            .providers
            .iter()
            .find(|provider| provider.supports(&request.capability))
        else {
            return Ok((
                PlanResponse::Unsupported {
                    reason: "no provider supports capability".into(),
                },
                None,
            ));
        };

        let plan = provider.plan(request)?;
        if let Err(error) = plan.validate() {
            return Ok((
                PlanResponse::Invalid {
                    reason: error.to_string(),
                },
                None,
            ));
        }

        let decision = self.policy.evaluate(&plan);
        Ok((PlanResponse::Planned(plan), Some(decision)))
    }
}

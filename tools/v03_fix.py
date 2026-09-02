from pathlib import Path

path = Path("crates/linura-control/src/review_projection.rs")
text = path.read_text(encoding="utf-8")
old = "        Actor, ActorId, ActorKind, CapabilityId, PlanId, PrincipalId, ProviderId, RequestId,\n        ResourceId, RiskClass, SemanticReason,"
new = "        Actor, ActorId, ActorKind, CapabilityId, IntentId, PlanId, PrincipalId, ProviderId,\n        RequestId, ResourceId, RiskClass, SemanticReason,"
if text.count(old) != 1:
    raise SystemExit(f"review projection import target count={text.count(old)}")
text = text.replace(old, new, 1)
old = "                intent_ids: vec![],\n                requirement_ids: vec![],"
new = "                intent_ids: vec![IntentId::new(\"intent:review-test\")\n                    .unwrap_or_else(|error| unreachable!(\"{error}\"))],\n                requirement_ids: vec![],"
if text.count(old) != 1:
    raise SystemExit(f"review projection origin target count={text.count(old)}")
path.write_text(text.replace(old, new, 1), encoding="utf-8")
print("v0.3 projection test provenance corrected")

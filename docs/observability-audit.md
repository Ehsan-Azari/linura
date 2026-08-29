# Observability, audit and semantic provenance

Linura separates operational telemetry, mutation audit and semantic provenance.

- **Telemetry/health**: is a component/provider healthy and performant?
- **Audit**: who requested/approved/executed what, when, with what result/evidence?
- **Semantic provenance**: why does the managed state exist and from which intent/requirement/capability was it derived?

A complete successful mutation eventually links request ID, actor, intent IDs, plan ID, policy decision/approval, effects, verification observations and provenance records. Failure/compensation records remain append-only.

`Explain` is generated from structured graph/provenance evidence, not from model memory.

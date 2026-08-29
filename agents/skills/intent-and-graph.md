# Intent and system-graph task guide

Intent is what the user wants; desired state is what the machine should be; the graph explains resources, dependencies, ownership, and why.

- Never erase semantic provenance when deriving state.
- Retirement must compute shared-resource impact before cleanup.
- Conversation text is input, not authoritative persisted state.
- Add negative tests for cycles, shared ownership, stale graph state, suspend/resume, and supersession lineage.
- Keep explainability machine-readable before generating natural-language explanations.

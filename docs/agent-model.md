# Agent model

This document is retained as the authority-side view; see `agent-architecture.md` for the intelligence runtime.

Agents are authenticated actors with explicit grants and are always untrusted proposers. An agent may:
- inspect allowed observed/desired state;
- request explanations;
- submit an `IntentProposal`;
- request a plan;
- ask for actions permitted by policy and approval.

An agent may not:
- execute arbitrary shell/root commands through Linura;
- bypass policy/approval;
- silently activate its own intent proposal;
- receive raw secret material by default;
- modify provenance/history;
- treat a model response as verification evidence.

Sensitive mutations require the same or stronger approval regardless of how persuasive/confident the model appears.

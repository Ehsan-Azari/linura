# Security model

## Trust boundaries

```text
natural language / imported content / external services
                       │
                model providers
                       │
               IntentProposal only
                       ▼
Linura First Boot / Linura Agent / Linura Control Center / CLI / Linura SDK / Extensions
                       │
                authenticated IPC
                       ▼
             ┌──────────────────┐
             │     linurad      │  unprivileged authority
             │ graph/plan/policy│
             └────────┬─────────┘
                      │ explicit typed privileged effect
                    Polkit
                      │
          ┌───────────▼───────────┐
          │ narrow root executors │
          └───────────┬───────────┘
                      │
               Linux subsystems
```

A compromised model provider is expected to be capable of proposing malicious intent. It must **not** be equivalent to compromise of the authority plane.

## Privileged executor rules

Each executor:
- owns one narrow domain;
- validates identifiers/arguments again at the privilege boundary;
- receives correlated request/plan context;
- performs no general orchestration or natural-language interpretation;
- exposes no arbitrary command/shell endpoint;
- uses native system APIs where available;
- returns structured outcome for independent verification;
- is separately sandboxed/hardened.

## Agent/model rules

- model output is untrusted data;
- agents emit typed `IntentProposal`/advice, not executable code;
- agent context is least-privilege and minimized;
- secrets use references/handles and are excluded from general prompts by default;
- prompt injection from web/files/repository content cannot create authority;
- tool/model/provider identity does not substitute for OS actor identity;
- sensitive mutations require policy/approval regardless of model confidence;
- offline/no-model behavior is a required security/recovery path.

## Local caller security

“Local” is not equivalent to trusted. D-Bus sender credentials bind calls to OS identities. Agent identity and grant scope are additional authenticated context, not caller-provided labels.

## State integrity

Intent, desired state, graph/provenance and audit persistence require migrations, crash consistency, idempotency and corruption detection. Observed state is always re-derived from Linux providers when truth matters for execution.

## Remote/fleet security

A future fleet gateway is a separate process/service with mutual authentication, explicit enrollment, revocation, replay resistance, scoped device identity, staged rollout and its own threat model. No remote listener is added to `linurad` as a shortcut.

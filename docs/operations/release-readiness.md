# Supported release readiness checklist

A version may not be called production-supported until all applicable items are proven.

## Architecture/security
- [ ] No unresolved high/critical threat-model findings.
- [ ] Privileged executor APIs reviewed independently.
- [ ] Policy denial/approval tests cover every privileged action class.
- [ ] Secret-handling review complete.
- [ ] Extension isolation proven or extensions excluded from release.

## Quality
- [ ] Unit/provider/integration/VM acceptance suites green on exact candidate SHA.
- [ ] Upgrade from previous supported release tested.
- [ ] Interrupted update/migration recovery tested.
- [ ] Reboot/suspend/resume relevant scenarios tested.
- [ ] Hardware support matrix published.

## Supply chain/release
- [ ] Dependencies locked/audited.
- [ ] Third-party CI actions pinned immutably.
- [ ] SBOM generated.
- [ ] Checksums generated and verified.
- [ ] Artifacts signed/attested.
- [ ] Published release assets independently reverified.
- [ ] Release tag immutable and bound to verified candidate.

## Operations
- [ ] Native break-glass recovery documented/tested.
- [ ] Diagnostic bundle redaction verified.
- [ ] Audit export/retention behavior documented.
- [ ] Known limitations and unsupported states published.

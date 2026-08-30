# First-boot product architecture

The signature Linura flow is:

> **What do you want this computer to become?**

The first-boot app runs after a minimal recoverable base is available. It discovers hardware/capabilities, collects user intent and constraints, produces an `IntentProposal`, resolves a machine profile, and shows a reviewable plan before mutation.

First boot can also adopt reusable setups or a portable machine profile from the user's Linura Library/export. Adoption is never direct application: Linura validates the bundle, resolves required local secret references, observes the target machine, resolves capabilities and generates a fresh reviewable plan.

## Required escape hatches

- continue with a deterministic default profile;
- skip agents/model setup entirely;
- open Control Center/CLI;
- reach TTY/recovery environment;
- browse/adopt locally stored reusable setups;
- import a portable setup/profile;
- restore a snapshot.

First boot therefore remains useful with no network or model provider. A hosted sync/catalog service is never required to reconstruct a machine from a locally available setup/profile export.

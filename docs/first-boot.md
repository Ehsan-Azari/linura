# First-boot product architecture

The signature Linura flow is:

> **What do you want this computer to become?**

The first-boot app runs after a minimal recoverable base is available. It discovers hardware/capabilities, collects user intent and constraints, produces an `IntentProposal`, resolves a machine profile, and shows a reviewable plan before mutation.

## Required escape hatches

- continue with a deterministic default profile;
- skip agents/model setup entirely;
- open Control Center/CLI;
- reach TTY/recovery environment;
- import a portable profile;
- restore a snapshot.

First boot therefore remains useful with no network or model provider.

# ADR 0011: Linura owns the product and code namespace

Status: Accepted

## Decision

Linura is the only proper-noun umbrella brand. The control plane remains an architectural concept implemented by `linura-control`; it does not receive a second product brand.

Public and system namespaces use Linura consistently: `linura-*`, `linurad`, `linuractl`, and `org.linura.*`.

`Linura OS` is reserved for the installable distribution product. Linura Control and the SDK remain capable of targeting supported non-Linura-OS Linux platform profiles.

## Rationale

A single namespace reduces user and contributor cognitive load, avoids duplicated compatibility/version identities, and preserves the ability to explain the architecture precisely without turning architecture terminology into separate brands.

## Consequences

- The authority crate is named `linura-control`.
- Developer-facing stable types are exposed through `linura-sdk` rather than requiring direct coupling to internal authority crates.
- Product application directories carry explicit Linura names.
- Architecture documentation uses “control plane” and “authority plane” as common nouns.

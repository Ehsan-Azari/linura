#!/usr/bin/env python3
from __future__ import annotations

from pathlib import Path
import re
import sys
import tomllib

VERSION_RE = re.compile(r"^v(\d+)\.(\d+)\.(\d+)$")
VALID_STATUS = {"released", "planned"}
VALID_CLAIM_CLASS = {"Experimental", "Preview", "Stable"}
VALID_EXECUTOR_STATE = {"none", "isolated-qualified", "integrated-narrow"}
VALID_MUTATION_SUPPORT = {"none", "narrow-experimental", "reference-stable"}
VALID_AGENT_ROLE = {"none", "proposal-only"}
VALID_PLATFORM_SUPPORT = {"none", "reference-experimental", "reference-stable"}
CANONICAL_LIFECYCLE = (
    "request/intent → observe → plan → validate → authorize → prepare → execute → verify → commit → audit → reconcile"
)


def version_key(value: str) -> tuple[int, int, int]:
    match = VERSION_RE.fullmatch(value)
    if match is None:
        raise ValueError(f"invalid semantic milestone version: {value!r}")
    return tuple(int(part) for part in match.groups())


def validate(root: Path) -> list[str]:
    failures: list[str] = []
    contract_path = root / "contracts/roadmap.toml"
    if not contract_path.is_file():
        return ["missing contracts/roadmap.toml"]

    try:
        contract = tomllib.loads(contract_path.read_text(encoding="utf-8"))
    except Exception as error:
        return [f"invalid contracts/roadmap.toml: {error}"]

    if contract.get("schema_version") != 1:
        failures.append("roadmap schema_version must be 1")
    if contract.get("product_stability") != "experimental":
        failures.append(
            "roadmap product_stability must describe the current product as experimental until a future explicit Stable release transition"
        )
    if contract.get("canonical_lifecycle") != CANONICAL_LIFECYCLE:
        failures.append(
            "roadmap canonical_lifecycle changed; the locked eleven-stage lifecycle requires an explicit architecture rebaseline"
        )

    milestones = contract.get("milestone")
    if not isinstance(milestones, list) or not milestones:
        return failures + ["roadmap contract must define at least one [[milestone]]"]

    required_fields = {
        "version",
        "title",
        "status",
        "claim_class",
        "depends_on",
        "durable_recovery",
        "executor_state",
        "complete_lifecycle",
        "managed_mutation_support",
        "agent_role",
        "platform_support",
    }

    by_version: dict[str, dict[str, object]] = {}
    ordered_versions: list[str] = []
    seen_planned = False

    for index, milestone in enumerate(milestones):
        if not isinstance(milestone, dict):
            failures.append(f"milestone #{index + 1} must be a table")
            continue

        missing = sorted(required_fields - milestone.keys())
        if missing:
            failures.append(f"milestone #{index + 1} missing fields: {missing}")
            continue

        version = milestone.get("version")
        title = milestone.get("title")
        status = milestone.get("status")
        claim_class = milestone.get("claim_class")
        depends_on = milestone.get("depends_on")
        durable_recovery = milestone.get("durable_recovery")
        executor_state = milestone.get("executor_state")
        complete_lifecycle = milestone.get("complete_lifecycle")
        mutation_support = milestone.get("managed_mutation_support")
        agent_role = milestone.get("agent_role")
        platform = milestone.get("platform_support")

        if not isinstance(version, str):
            failures.append(f"milestone #{index + 1} version must be a string")
            continue
        try:
            version_key(version)
        except ValueError as error:
            failures.append(str(error))
            continue
        if version in by_version:
            failures.append(f"duplicate roadmap milestone: {version}")
            continue
        if not isinstance(title, str) or not title.strip():
            failures.append(f"{version}: title must be a non-empty string")
        if status not in VALID_STATUS:
            failures.append(f"{version}: unsupported status {status!r}")
        if claim_class not in VALID_CLAIM_CLASS:
            failures.append(f"{version}: unsupported claim_class {claim_class!r}")
        if not isinstance(depends_on, list) or not all(isinstance(item, str) for item in depends_on):
            failures.append(f"{version}: depends_on must be an array of milestone versions")
        if not isinstance(durable_recovery, bool):
            failures.append(f"{version}: durable_recovery must be boolean")
        if executor_state not in VALID_EXECUTOR_STATE:
            failures.append(f"{version}: unsupported executor_state {executor_state!r}")
        if not isinstance(complete_lifecycle, bool):
            failures.append(f"{version}: complete_lifecycle must be boolean")
        if mutation_support not in VALID_MUTATION_SUPPORT:
            failures.append(f"{version}: unsupported managed_mutation_support {mutation_support!r}")
        if agent_role not in VALID_AGENT_ROLE:
            failures.append(f"{version}: unsupported agent_role {agent_role!r}")
        if platform not in VALID_PLATFORM_SUPPORT:
            failures.append(f"{version}: unsupported platform_support {platform!r}")

        if status == "planned":
            seen_planned = True
        elif status == "released" and seen_planned:
            failures.append(f"{version}: released milestones must form a contiguous prefix")

        by_version[version] = milestone
        ordered_versions.append(version)

    try:
        ordered_keys = [version_key(version) for version in ordered_versions]
        if ordered_keys != sorted(ordered_keys) or len(set(ordered_keys)) != len(ordered_keys):
            failures.append("roadmap milestones must be strictly increasing by semantic version")
    except ValueError:
        pass

    for version, milestone in by_version.items():
        depends_on = milestone.get("depends_on")
        if not isinstance(depends_on, list):
            continue
        for dependency in depends_on:
            if dependency not in by_version:
                failures.append(f"{version}: unknown dependency {dependency}")
                continue
            try:
                if version_key(dependency) >= version_key(version):
                    failures.append(f"{version}: dependency {dependency} must precede the milestone")
            except ValueError:
                continue

    released = [
        version
        for version in ordered_versions
        if by_version.get(version, {}).get("status") == "released"
    ]
    planned = [
        version
        for version in ordered_versions
        if by_version.get(version, {}).get("status") == "planned"
    ]

    current_release = contract.get("current_release")
    next_release = contract.get("next_release")
    if not released:
        failures.append("roadmap must identify at least one released milestone")
    elif current_release != released[-1]:
        failures.append(
            f"current_release must equal the last released milestone {released[-1]}, found {current_release!r}"
        )
    if not planned:
        failures.append("roadmap must identify at least one planned milestone")
    elif next_release != planned[0]:
        failures.append(
            f"next_release must equal the first planned milestone {planned[0]}, found {next_release!r}"
        )

    canonical_document = contract.get("canonical_document")
    domain_document = contract.get("domain_document")
    versioning_document = contract.get("versioning_document")

    if not isinstance(canonical_document, str):
        failures.append("canonical_document must be a string path")
        roadmap_text = ""
    else:
        roadmap_path = root / canonical_document
        if not roadmap_path.is_file():
            failures.append(f"canonical roadmap document missing: {canonical_document}")
            roadmap_text = ""
        else:
            roadmap_text = roadmap_path.read_text(encoding="utf-8")

    if not isinstance(domain_document, str):
        failures.append("domain_document must be a string path")
        domain_text = ""
    else:
        domain_path = root / domain_document
        if not domain_path.is_file():
            failures.append(f"domain roadmap document missing: {domain_document}")
            domain_text = ""
        else:
            domain_text = domain_path.read_text(encoding="utf-8")

    if not isinstance(versioning_document, str):
        failures.append("versioning_document must be a string path")
        versioning_text = ""
    else:
        versioning_path = root / versioning_document
        if not versioning_path.is_file():
            failures.append(f"versioning policy document missing: {versioning_document}")
            versioning_text = ""
        else:
            versioning_text = versioning_path.read_text(encoding="utf-8")

    for version, milestone in by_version.items():
        title = milestone.get("title")
        if isinstance(title, str) and roadmap_text:
            heading = f"## {version} — {title}"
            if heading not in roadmap_text:
                failures.append(f"canonical roadmap missing exact heading: {heading}")

        if milestone.get("status") == "released":
            release_contract = milestone.get("release_contract")
            if not isinstance(release_contract, str):
                failures.append(f"{version}: released milestone must name release_contract")
            elif not (root / release_contract).is_file():
                failures.append(f"{version}: release contract does not exist: {release_contract}")

        qualification = milestone.get("qualification")
        if qualification is not None:
            if not isinstance(qualification, str) or not (root / qualification).is_file():
                failures.append(f"{version}: qualification document does not exist: {qualification!r}")

    def dependency_closure(version: str) -> set[str]:
        visited: set[str] = set()
        stack = list(by_version.get(version, {}).get("depends_on", []))
        while stack:
            dependency = stack.pop()
            if not isinstance(dependency, str) or dependency in visited:
                continue
            visited.add(dependency)
            nested = by_version.get(dependency, {}).get("depends_on", [])
            if isinstance(nested, list):
                stack.extend(nested)
        return visited

    # These are deliberate architectural gates, not estimates. Changing them requires
    # an explicit roadmap rebaseline rather than silently moving product authority earlier.
    expected_gates = {
        "v0.0.0": (False, "none", False, "none", "none", "none"),
        "v0.1.0": (False, "none", False, "none", "none", "none"),
        "v0.2.0": (False, "none", False, "none", "none", "none"),
        "v0.3.0": (False, "none", False, "none", "none", "none"),
        "v0.4.0": (True, "none", False, "none", "none", "none"),
        "v0.5.0": (True, "isolated-qualified", False, "none", "none", "none"),
        "v0.6.0": (True, "integrated-narrow", True, "narrow-experimental", "none", "none"),
        "v0.7.0": (True, "integrated-narrow", True, "narrow-experimental", "none", "none"),
        "v0.8.0": (True, "integrated-narrow", True, "narrow-experimental", "proposal-only", "none"),
        "v0.9.0": (
            True,
            "integrated-narrow",
            True,
            "narrow-experimental",
            "proposal-only",
            "reference-experimental",
        ),
        "v0.10.0": (
            True,
            "integrated-narrow",
            True,
            "narrow-experimental",
            "proposal-only",
            "reference-experimental",
        ),
        "v1.0.0": (
            True,
            "integrated-narrow",
            True,
            "reference-stable",
            "proposal-only",
            "reference-stable",
        ),
    }
    if set(by_version) != set(expected_gates):
        failures.append(
            "roadmap milestone set changed; perform an explicit roadmap-contract rebaseline and update checker gates"
        )
    for version, expected in expected_gates.items():
        milestone = by_version.get(version)
        if milestone is None:
            continue
        actual = (
            milestone.get("durable_recovery"),
            milestone.get("executor_state"),
            milestone.get("complete_lifecycle"),
            milestone.get("managed_mutation_support"),
            milestone.get("agent_role"),
            milestone.get("platform_support"),
        )
        if actual != expected:
            failures.append(f"{version}: architectural gate changed from {expected} to {actual}")

    for version, milestone in by_version.items():
        closure = dependency_closure(version)
        durable_recovery = milestone.get("durable_recovery") is True
        executor_state = milestone.get("executor_state")
        complete_lifecycle = milestone.get("complete_lifecycle") is True
        mutation_support = milestone.get("managed_mutation_support")
        platform_support = milestone.get("platform_support")
        claim_class = milestone.get("claim_class")

        if durable_recovery and version != "v0.4.0" and "v0.4.0" not in closure:
            failures.append(f"{version}: durable recovery requires the v0.4.0 foundation")
        if executor_state in {"isolated-qualified", "integrated-narrow"} and not durable_recovery:
            failures.append(f"{version}: executor qualification requires durable recovery semantics")
        if executor_state == "integrated-narrow" and "v0.5.0" not in closure:
            failures.append(f"{version}: integrated executor requires v0.5.0 isolated executor/verifier qualification")
        if complete_lifecycle:
            if not durable_recovery or executor_state != "integrated-narrow":
                failures.append(f"{version}: complete lifecycle requires durable recovery and integrated narrow executor")
            if version != "v0.6.0" and "v0.6.0" not in closure:
                failures.append(f"{version}: complete lifecycle requires the v0.6.0 integration milestone")
        if mutation_support != "none":
            if not complete_lifecycle:
                failures.append(f"{version}: supported managed mutation requires complete lifecycle proof")
            if executor_state != "integrated-narrow":
                failures.append(f"{version}: supported managed mutation requires an integrated narrow executor")
            if not durable_recovery:
                failures.append(f"{version}: supported managed mutation requires durable recovery")
            if version != "v0.6.0" and "v0.6.0" not in closure:
                failures.append(f"{version}: supported managed mutation cannot precede v0.6.0")
        if mutation_support == "reference-stable":
            if claim_class != "Stable":
                failures.append(f"{version}: Stable mutation support requires a Stable milestone claim")
            if platform_support != "reference-stable":
                failures.append(f"{version}: Stable mutation support requires a Stable reference platform")
            if "v0.10.0" not in closure:
                failures.append(f"{version}: Stable mutation support requires the v0.10.0 Experimental end-user milestone")
        if milestone.get("agent_role") == "proposal-only" and "v0.7.0" not in closure:
            failures.append(f"{version}: agent interpretation requires the persistent trusted core through v0.7.0")
        if platform_support == "reference-experimental" and "v0.8.0" not in closure:
            failures.append(f"{version}: Experimental reference platform support requires the v0.8.0 proposal boundary")
        if platform_support == "reference-stable":
            if claim_class != "Stable":
                failures.append(f"{version}: Stable reference platform support requires a Stable milestone claim")
            if "v0.10.0" not in closure:
                failures.append(f"{version}: Stable reference platform support requires v0.10.0 experience evidence")

    v010 = by_version.get("v0.10.0")
    if v010 is not None and v010.get("claim_class") != "Experimental":
        failures.append("v0.10.0 must remain the explicitly Experimental end-user milestone")

    v1 = by_version.get("v1.0.0")
    if v1 is not None:
        if v1.get("claim_class") != "Stable":
            failures.append("v1.0.0 is reserved for the first Stable supported end-user contract")
        if v1.get("depends_on") != ["v0.10.0"]:
            failures.append("v1.0.0 must follow the v0.10.0 Experimental end-user milestone")

    required_versioning_markers = (
        "`v1.0.0` is the first stable end-user contract.",
        "The 1.0 release contract must have evidence appropriate to a stable system layer",
        "After 1.0, normal Semantic Versioning applies",
    )
    for marker in required_versioning_markers:
        if versioning_text and marker not in versioning_text:
            failures.append(f"versioning policy missing Stable v1 invariant: {marker}")

    required_roadmap_markers = (
        "## v0.10.0 — meaningful end-user Experimental Linura",
        "## v1.0.0 — first Stable supported end-user Linura",
        "## Beyond v1.0 — broader support and product expansion",
        "## Post-v1 strategic tracks",
        "### Personal operating environment",
        "### Extension and sharing ecosystem",
        "### General-purpose provider breadth",
        "### Optional fleet and enterprise",
        "## Independent maturity axes",
        "## VM and virtualization boundary",
        "## Dependency gates",
        "## Anti-drift governance",
        "## Roadmap-change procedure",
        "Code presence is not support",
        "Models are untrusted proposers",
        "v0.5 may exercise a narrow executor/verifier only through disposable qualification authority",
        "no supported managed external mutation may appear before v0.6",
        "v1.0 is reserved for the first Stable supported end-user contract",
        CANONICAL_LIFECYCLE,
    )
    for marker in required_roadmap_markers:
        if roadmap_text and marker not in roadmap_text:
            failures.append(f"canonical roadmap missing governance marker: {marker}")

    required_domain_markers = (
        "## VM qualification versus VM management",
        "test infrastructure, not a product virtualization capability",
        "libvirt/QEMU/KVM, Incus",
        "→ validate\n→ authorize\n→ prepare\n→ execute through a narrow provider executor\n→ verify through independent re-observation\n→ commit\n→ audit\n→ reconcile",
    )
    for marker in required_domain_markers:
        if domain_text and marker not in domain_text:
            failures.append(f"system domain map missing virtualization boundary marker: {marker}")

    return failures


def main(argv: list[str]) -> int:
    root = Path(argv[1]).resolve() if len(argv) > 1 else Path(__file__).resolve().parents[1]
    failures = validate(root)
    if failures:
        for failure in failures:
            print(f"ERROR: {failure}", file=sys.stderr)
        return 1
    print("roadmap contract checks passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))

#!/usr/bin/env python3
from __future__ import annotations

from pathlib import Path
import re
import sys
import tomllib

EXPECTED_AUTHORITY_STATES = {
    "v0.0.0": "none",
    "v0.1.0": "none",
    "v0.2.0": "none",
    "v0.3.0": "review-only",
    "v0.4.0": "durable-reviewed",
    "v0.5.0": "durable-reviewed",
    "v0.6.0": "lifecycle-integrated",
    "v0.7.0": "lifecycle-integrated",
    "v0.8.0": "lifecycle-integrated",
    "v0.9.0": "lifecycle-integrated",
    "v0.10.0": "lifecycle-integrated",
    "v1.0.0": "lifecycle-integrated",
}
VALID_AUTHORITY_STATES = {"none", "review-only", "durable-reviewed", "lifecycle-integrated"}

REMOVED_BOOTSTRAP_MARKERS = {
    "crates/linura-core/src/lib.rs": (
        "pub struct ActionPlan",
        "pub struct Effect",
        "pub enum Compensation",
    ),
    "crates/linura-protocol/src/lib.rs": (
        "pub struct ActionRequest",
        "pub enum PlanResponse",
    ),
    "crates/linura-provider-sdk/src/lib.rs": (
        "pub struct Observation {",
        "pub trait Provider:",
        "pub trait EffectExecutor",
        "pub trait EffectVerifier",
    ),
    "crates/linura-control/src/lib.rs": (
        "pub trait MutationRuntime",
        "pub struct ControlPlane<",
        "pub fn apply<",
    ),
}

PRESERVED_SCAFFOLD_MARKERS = {
    "crates/linura-lifecycle/src/lib.rs": (
        "pub const MUTATION_STAGES: [MutationStage; 11]",
        "MutationStage::Authorize",
        "MutationStage::Prepare",
        "MutationStage::Execute",
        "MutationStage::Verify",
        "MutationStage::Commit",
        "MutationStage::Audit",
        "MutationStage::Reconcile",
    ),
    "executors/linura-executor-systemd/src/lib.rs": (
        "pub enum SystemdOperation",
        "SetUnitEnabled",
        "RestartUnit",
    ),
    "crates/linura-provider-sdk/src/lib.rs": (
        "pub trait Observer",
        "Result<ObservationEnvelope, ProviderError>",
    ),
    "crates/linura-planner/src/lib.rs": (
        "pub struct ReconciliationPlan",
        "ExecutionAuthority::Disabled",
    ),
    "crates/linura-control/src/risk_classification.rs": (
        "BASELINE_RISK_POLICY_REVISION",
        "return RiskClassification::Unclassified {",
        "return RiskClassification::DowngradeRejected {",
        "systemd.unit.active-state.security-sensitive",
        "no trusted risk rule covers",
    ),
    "crates/linura-control/src/policy_review.rs": (
        "classify_plan_risk(plan)",
        "code: \"authority-risk-classified\".into()",
        "code: \"authority-risk-unclassified\".into()",
        "code: \"authority-risk-downgrade-rejected\".into()",
    ),
}

EXPECTED_APPROVAL_BY_RISK = {
    "SystemMutation": "InteractiveUser",
    "SecuritySensitive": "Administrator",
    "Destructive": "DestructiveAction",
}

REQUIRED_V03_MARKERS = {
    "docs/milestones/v0.3.0.md": (
        "v0.3 extends the v0.2 `linura-planner::ReconciliationPlan` lineage.",
        "The planner's `prospective_risk` is a risk floor.",
        "Unclassified mutation risk fails closed as `blocked`",
        "Approval does not create execution authority.",
        "A reviewed plan does not create a durable prepare record.",
    ),
    "docs/qualification/v0.3.0.md": (
        "qualification specification",
        "risk-policy revision and matched rule identities",
        "risk downgrade below the planner floor is rejected",
        "A plan ID by itself is not sufficient authority evidence.",
        "native Linux state remains unchanged",
    ),
    "docs/adr/0018-canonical-plan-review-authority.md": (
        "The planner's `prospective_risk` is a lower bound",
        "Unknown mutation shapes fail closed",
        "policy allow          != execution authority",
        "valid approval        != execution authority",
        "reviewed plan         != prepared mutation",
    ),
    "docs/threat-model.md": (
        "### Risk downgrade, under-classification or classifier substitution",
        "no matching trusted rule means unclassified mutation risk",
        "### Approval replay, theft or policy substitution",
        "cross-principal approval reuse",
    ),
}


def read_text(root: Path, relative: str, failures: list[str]) -> str:
    path = root / relative
    if not path.is_file():
        failures.append(f"required authority-foundation file missing: {relative}")
        return ""
    return path.read_text(encoding="utf-8")


def _sanitize_rust_source(text: str) -> str:
    """Mask Rust comments and string contents while preserving source structure."""
    output: list[str] = []
    index = 0
    block_depth = 0
    in_line_comment = False
    in_string = False
    escaped = False

    while index < len(text):
        char = text[index]
        next_char = text[index + 1] if index + 1 < len(text) else ""

        if in_line_comment:
            if char == "\n":
                in_line_comment = False
                output.append("\n")
            else:
                output.append(" ")
            index += 1
            continue

        if block_depth:
            if char == "/" and next_char == "*":
                block_depth += 1
                output.extend((" ", " "))
                index += 2
                continue
            if char == "*" and next_char == "/":
                block_depth -= 1
                output.extend((" ", " "))
                index += 2
                continue
            output.append("\n" if char == "\n" else " ")
            index += 1
            continue

        if in_string:
            if escaped:
                escaped = False
                output.append(" ")
            elif char == "\\":
                escaped = True
                output.append(" ")
            elif char == '"':
                in_string = False
                output.append('"')
            else:
                output.append("\n" if char == "\n" else " ")
            index += 1
            continue

        if char == "/" and next_char == "/":
            in_line_comment = True
            output.extend((" ", " "))
            index += 2
            continue
        if char == "/" and next_char == "*":
            block_depth = 1
            output.extend((" ", " "))
            index += 2
            continue
        if char == '"':
            in_string = True
            output.append('"')
            index += 1
            continue

        output.append(char)
        index += 1

    return "".join(output)


def _extract_braced_block(text: str, marker: str) -> tuple[str, int] | None:
    marker_index = text.find(marker)
    if marker_index < 0:
        return None
    open_index = text.find("{", marker_index + len(marker))
    if open_index < 0:
        return None

    depth = 0
    for index in range(open_index, len(text)):
        char = text[index]
        if char == "{":
            depth += 1
        elif char == "}":
            depth -= 1
            if depth == 0:
                return text[open_index + 1 : index], index + 1
    return None


def validate_policy_approval_strength(policy_text: str) -> list[str]:
    failures: list[str] = []
    source = _sanitize_rust_source(policy_text)

    impl_block = _extract_braced_block(source, "impl PolicyEngine for BaselinePolicy")
    if impl_block is None:
        return ["BaselinePolicy PolicyEngine implementation is missing or malformed"]

    impl_body, _ = impl_block
    function_block = _extract_braced_block(
        impl_body,
        "fn evaluate_decision(&self, subject: &PolicySubject) -> PolicyDecision",
    )
    if function_block is None:
        return ["BaselinePolicy::evaluate_decision is missing or malformed"]

    function_body, _ = function_block
    risk_match_marker = "match subject.prospective_risk()"
    if function_body.count(risk_match_marker) != 1:
        failures.append(
            "BaselinePolicy::evaluate_decision must contain exactly one canonical prospective-risk match"
        )
        return failures

    match_index = function_body.index(risk_match_marker)
    if "subject.prospective_risk()" in function_body[:match_index]:
        failures.append(
            "BaselinePolicy::evaluate_decision must not short-circuit on prospective risk before the canonical risk match"
        )

    risk_match = _extract_braced_block(function_body, risk_match_marker)
    if risk_match is None:
        failures.append("canonical prospective-risk match is malformed")
        return failures

    match_body, match_end = risk_match
    if function_body[match_end:].strip():
        failures.append(
            "canonical prospective-risk match must remain the tail decision expression"
        )

    for risk_name, expected_class in EXPECTED_APPROVAL_BY_RISK.items():
        arm_marker = f"RiskClass::{risk_name} => PolicyDecision::RequireApproval"
        if match_body.count(arm_marker) != 1:
            failures.append(
                f"canonical risk match must contain exactly one {risk_name} RequireApproval arm"
            )
            continue

        arm_block = _extract_braced_block(match_body, arm_marker)
        if arm_block is None:
            failures.append(f"{risk_name} approval arm is malformed")
            continue
        arm_body, _ = arm_block
        classes = re.findall(r"class\s*:\s*ApprovalClass::([A-Za-z0-9_]+)", arm_body)
        if classes != [expected_class]:
            failures.append(
                f"{risk_name} must require exactly ApprovalClass::{expected_class}; found {classes!r}"
            )

    return failures


def validate(root: Path) -> list[str]:
    failures: list[str] = []
    contract_path = root / "contracts/roadmap.toml"
    if not contract_path.is_file():
        return ["missing contracts/roadmap.toml"]

    try:
        contract = tomllib.loads(contract_path.read_text(encoding="utf-8"))
    except (OSError, tomllib.TOMLDecodeError) as error:
        return [f"invalid contracts/roadmap.toml: {error}"]

    milestones = contract.get("milestone")
    if not isinstance(milestones, list):
        return ["roadmap contract milestone must be a list"]

    by_version: dict[str, dict[str, object]] = {}
    for milestone in milestones:
        if not isinstance(milestone, dict):
            failures.append("roadmap milestone must be a table")
            continue
        version = milestone.get("version")
        if not isinstance(version, str):
            failures.append("roadmap milestone version must be a string")
            continue
        by_version[version] = milestone
        authority_state = milestone.get("authority_state")
        if authority_state not in VALID_AUTHORITY_STATES:
            failures.append(f"{version}: unsupported authority_state {authority_state!r}")

    if set(by_version) != set(EXPECTED_AUTHORITY_STATES):
        failures.append(
            "authority milestone set changed; explicitly rebaseline authority-state expectations"
        )
    for version, expected in EXPECTED_AUTHORITY_STATES.items():
        milestone = by_version.get(version)
        if milestone is None:
            continue
        actual = milestone.get("authority_state")
        if actual != expected:
            failures.append(
                f"{version}: authority_state must remain {expected!r}, found {actual!r}"
            )

    v03 = by_version.get("v0.3.0")
    if v03 is not None:
        if v03.get("durable_recovery") is not False:
            failures.append("v0.3.0 review-only authority must not claim durable recovery")
        if v03.get("executor_state") != "none":
            failures.append("v0.3.0 review-only authority must not carry an executor")
        if v03.get("complete_lifecycle") is not False:
            failures.append("v0.3.0 review-only authority must not claim complete lifecycle")
        if v03.get("managed_mutation_support") != "none":
            failures.append("v0.3.0 review-only authority must not support managed mutation")
        if v03.get("milestone_contract") != "docs/milestones/v0.3.0.md":
            failures.append("v0.3.0 must bind docs/milestones/v0.3.0.md")

    for version in ("v0.4.0", "v0.5.0"):
        milestone = by_version.get(version)
        if milestone is not None and milestone.get("durable_recovery") is not True:
            failures.append(f"{version}: durable-reviewed authority requires durable recovery")

    for version in (
        "v0.6.0",
        "v0.7.0",
        "v0.8.0",
        "v0.9.0",
        "v0.10.0",
        "v1.0.0",
    ):
        milestone = by_version.get(version)
        if milestone is None:
            continue
        if milestone.get("complete_lifecycle") is not True:
            failures.append(f"{version}: lifecycle-integrated authority requires complete lifecycle")
        if milestone.get("executor_state") != "integrated-narrow":
            failures.append(f"{version}: lifecycle-integrated authority requires integrated narrow executor")

    for relative, markers in REMOVED_BOOTSTRAP_MARKERS.items():
        text = read_text(root, relative, failures)
        for marker in markers:
            if marker in text:
                failures.append(
                    f"superseded bootstrap authority marker reintroduced in {relative}: {marker}"
                )

    for relative, markers in PRESERVED_SCAFFOLD_MARKERS.items():
        text = read_text(root, relative, failures)
        for marker in markers:
            if marker not in text:
                failures.append(f"future authority scaffold missing from {relative}: {marker}")

    policy_text = read_text(root, "crates/linura-policy/src/lib.rs", failures)
    failures.extend(validate_policy_approval_strength(policy_text))

    for relative, markers in REQUIRED_V03_MARKERS.items():
        text = read_text(root, relative, failures)
        for marker in markers:
            if marker not in text:
                failures.append(f"v0.3 authority contract marker missing from {relative}: {marker}")

    return failures


def main(argv: list[str]) -> int:
    root = Path(argv[1]).resolve() if len(argv) > 1 else Path(__file__).resolve().parents[1]
    failures = validate(root)
    if failures:
        for failure in failures:
            print(f"ERROR: {failure}", file=sys.stderr)
        return 1
    print("authority foundation checks passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))

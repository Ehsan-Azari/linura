from pathlib import Path

path = Path("tests/tooling/test_dbus_contract.py")
text = path.read_text(encoding="utf-8")
old = '''PLAN_PREVIEW_OUTPUTS = (
    ("ids", "(ss)", "out"),
    ("actor", "(ssb)", "out"),
    ("route", "(sss)", "out"),
    ("reason", "(sasasas)", "out"),
    ("observed_evidence_id", "s", "out"),
    ("prospective_risk", "s", "out"),
    ("status", "s", "out"),
    ("execution_authorized", "b", "out"),
    ("changes", "a(sbss)", "out"),
    ("findings", "a(sss)", "out"),
)
'''
new = old + '''
PLAN_REVIEW_OUTPUTS = (
    ("ids", "(ss)", "out"),
    ("principal", "s", "out"),
    ("actor", "(ssb)", "out"),
    ("route", "(sss)", "out"),
    ("reason", "(sasasas)", "out"),
    ("observed_evidence_id", "s", "out"),
    ("review", "(sssss)", "out"),
    ("decision", "(sbssb)", "out"),
    ("changes", "a(sbss)", "out"),
    ("findings", "a(sss)", "out"),
)
'''
if text.count(old) != 1:
    raise SystemExit(f"PLAN_PREVIEW_OUTPUTS target count={text.count(old)}")
text = text.replace(old, new, 1)
old = '    "ExplainPlanPreview": (("plan_id", "s", "in"), *PLAN_PREVIEW_OUTPUTS),\n}'
new = '    "ExplainPlanPreview": (("plan_id", "s", "in"), *PLAN_PREVIEW_OUTPUTS),\n    "ReviewPlan": (("plan_id", "s", "in"), *PLAN_REVIEW_OUTPUTS),\n    "ExplainPlanReview": (("plan_id", "s", "in"), *PLAN_REVIEW_OUTPUTS),\n}'
if text.count(old) != 1:
    raise SystemExit(f"EXPECTED_METHODS target count={text.count(old)}")
text = text.replace(old, new, 1)
old = '    "ExplainPlanPreview": "async fn explain_plan_preview(",\n}'
new = '    "ExplainPlanPreview": "async fn explain_plan_preview(",\n    "ReviewPlan": "async fn review_plan(",\n    "ExplainPlanReview": "async fn explain_plan_review(",\n}'
if text.count(old) != 1:
    raise SystemExit(f"RUNTIME_METHODS target count={text.count(old)}")
text = text.replace(old, new, 1)
old = '        self.assertTrue({"PlanDesiredState", "GetPlanPreview", "ExplainPlanPreview"} <= method_names)'
new = '        self.assertTrue(\n            {\n                "PlanDesiredState",\n                "GetPlanPreview",\n                "ExplainPlanPreview",\n                "ReviewPlan",\n                "ExplainPlanReview",\n            }\n            <= method_names\n        )'
if text.count(old) != 1:
    raise SystemExit(f"non-executable surface target count={text.count(old)}")
text = text.replace(old, new, 1)
path.write_text(text, encoding="utf-8")
print("v0.3 canonical D-Bus contract expectation updated")

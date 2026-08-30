from __future__ import annotations

from pathlib import Path
import unittest
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[2]
XML_PATH = ROOT / "interfaces/dbus/org.linura.Control1.xml"
RUNTIME_PATH = ROOT / "crates/linura-dbus/src/lib.rs"

EXPECTED_METHODS = {
    "WhoAmI": (
        ("actor_id", "s", "out"),
        ("actor_kind", "s", "out"),
        ("interactive", "b", "out"),
        ("uid", "u", "out"),
        ("pid", "u", "out"),
        ("dbus_sender", "s", "out"),
    ),
    "Capabilities": (
        ("providers", "a(sss)", "out"),
        ("capabilities", "a(ssss)", "out"),
    ),
    "Observe": (
        ("provider", "s", "in"),
        ("resource", "s", "in"),
        ("capability", "s", "in"),
        ("observed_provider", "s", "out"),
        ("observed_resource", "s", "out"),
        ("observed_capability", "s", "out"),
        ("authority", "s", "out"),
        ("freshness", "s", "out"),
        ("observed_at_unix_ms", "t", "out"),
        ("valid_for_ms", "t", "out"),
        ("sequence", "t", "out"),
        ("attributes", "a(ss)", "out"),
    ),
    "Graph": (
        ("nodes", "a(sa(ss))", "out"),
        ("edges", "a(ssss)", "out"),
    ),
    "Explain": (
        ("resource", "s", "in"),
        ("explained_resource", "s", "out"),
        ("provider", "s", "out"),
        ("capability", "s", "out"),
        ("freshness", "s", "out"),
        ("evidence_id", "s", "out"),
        ("authority", "s", "out"),
    ),
}

RUNTIME_METHODS = {
    "WhoAmI": "async fn who_am_i(",
    "Capabilities": "async fn capabilities(",
    "Observe": "async fn observe(",
    "Graph": "async fn graph(",
    "Explain": "async fn explain(",
}

FORBIDDEN_MUTATION_METHODS = {"ProposeIntent", "Plan", "Commit", "Execute", "Apply"}


class Control1ContractTests(unittest.TestCase):
    def test_introspection_contract_is_exact_and_read_only(self) -> None:
        root = ET.parse(XML_PATH).getroot()
        interface = root.find("./interface[@name='org.linura.Control1']")
        self.assertIsNotNone(interface)
        assert interface is not None

        actual: dict[str, tuple[tuple[str, str, str], ...]] = {}
        for method in interface.findall("method"):
            name = method.attrib["name"]
            args = tuple(
                (
                    arg.attrib["name"],
                    arg.attrib["type"],
                    arg.attrib.get("direction", "in"),
                )
                for arg in method.findall("arg")
            )
            actual[name] = args

        self.assertEqual(actual, EXPECTED_METHODS)
        self.assertTrue(FORBIDDEN_MUTATION_METHODS.isdisjoint(actual))

    def test_runtime_exports_every_declared_control_method(self) -> None:
        source = RUNTIME_PATH.read_text(encoding="utf-8")
        for method, rust_marker in RUNTIME_METHODS.items():
            with self.subTest(method=method):
                self.assertIn(rust_marker, source)

        for forbidden in ("propose_intent", "plan", "commit", "execute", "apply"):
            with self.subTest(forbidden=forbidden):
                self.assertNotIn(f"async fn {forbidden}(", source)


if __name__ == "__main__":
    unittest.main()

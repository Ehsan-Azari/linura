# Acceptance scenarios

Each JSON file is a versioned disposable-machine scenario validated against `schemas/acceptance-scenario.v1.schema.json`.

Run discovery with:

```bash
python3 tools/acceptance.py list
```

A scenario should assert user/system-observable behavior and recovery properties. Do not add a passing placeholder for a feature that has not been implemented; scenarios may exist as reserved development contracts, but release evidence must name the image digest and successful execution run.

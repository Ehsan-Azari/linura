# Workflow model

Linura can compose missing workflows from typed capabilities. Example:

```text
Super+Shift+S
 → capture region
 → annotate
 → upload through selected storage capability
 → copy URL
 → notify
```

Workflows are declarative graphs of typed triggers/actions. They do not embed arbitrary privileged shell text. Their resources and permissions participate in the same system graph, provenance and retirement lifecycle as other managed capabilities.

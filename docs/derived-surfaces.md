# Derived UI surfaces

Linura may create or install UI for newly composed capabilities, but models must not directly install arbitrary privileged GUI code.

The preferred path is a constrained surface description over typed resources/actions, rendered by trusted UI components. Examples include resource tables, details, forms, status cards and safe action controls.

Truly custom UI executes through the isolated extension model and requests explicit capabilities. UI remains a client of the authority API and never becomes an alternate backend.

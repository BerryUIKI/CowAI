# Documentation

Start with the root [README](../README.md) for implemented functionality and a runnable quick start.

| Audience | Guide |
| --- | --- |
| Developers | [Development and troubleshooting](DEVELOPMENT.md) |
| Contributors | [Contributing](../CONTRIBUTING.md) |
| Collaborators and reviewers | [Collaboration workflow](COLLABORATION.md) |
| Maintainers | [Release checklist](RELEASING.md) |
| Community | [Code of Conduct](../CODE_OF_CONDUCT.md) and [security reporting](../SECURITY.md) |
| Product contributors | [PRD](PRD.md) and [roadmap](ROADMAP.md) |
| Architecture contributors | [Target architecture](ARCHITECTURE.md) and [decision records](ADR/README.md) |

## Reading design documents

The PRD, architecture, roadmap, and ADRs include both implemented milestones and future scope. Milestones 1.1–1.5 and parts of 1.6 are implemented: persistence, services, approvals, a local client, CLI integration, and a desktop shell. Check the root README and source for precise availability; starting execution from the desktop remains follow-up work.

## Visual assets

[Runtime architecture](assets/architecture.svg) depicts current code. [Workspace concept](assets/workspace-concept.svg) illustrates the workspace direction and must retain its concept label; it is not an application screenshot. Both ship a `-light.svg` counterpart for light backgrounds; the README serves each through a `<picture>` element so the diagram follows the reader's color scheme. SVGs are editable source assets with accessible titles and descriptions; keep them self-contained, readable, and free of embedded scripts or external resources. Update the README alt text and both variants when changing their meaning.

Additional references: [Desktop setup](../desktop/README.md), [template format](TEMPLATE_FORMAT.md), [design references](REFERENCES.md), and [brand assets](../assets/README.md).

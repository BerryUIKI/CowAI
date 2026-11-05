# Collaboration workflow

## Branches and pull requests

CowBot follows GitFlow with `dev` (not `develop`) as its integration branch.

| Branch | Start from | Merge into | Purpose |
| --- | --- | --- | --- |
| `main` | — | — | Release-ready history |
| `dev` | — | — | Completed work awaiting release |
| `feature/<name>` | `dev` | `dev` | Features, fixes, and documentation |
| `release/<version>` | `dev` | `main` and `dev` | Release stabilization |
| `hotfix/<name>` | `main` | `main` and `dev` | Urgent release fixes |

Use non-fast-forward merges for feature integration. Ordinary PRs target `dev`; release and hotfix work must reach both long-lived branches. Contributors without write access use forks. Do not force-push shared long-lived branches. The repository conventions require PRs for both protected branches. Never push directly to `dev` or `main`; see [CLAUDE.md](../CLAUDE.md).

```sh
git fetch origin
git switch dev
git pull --ff-only origin dev
git switch -c feature/clarify-cli-docs
```

For a fork, fetch the canonical repository through an `upstream` remote and start from `upstream/dev`. If the integration branch is unavailable, ask maintainers in the issue which base to use.

## Coordinate work

Use issues for problem statements, ownership, acceptance criteria, and design discussion. Link PRs to their issues. Keep scope reviewable and surface dependencies or blocking questions early. Record lasting architecture decisions in ADRs rather than leaving them only in comments.

Reviewers should verify that implemented features and planned capabilities are described separately. Pay special attention to model credentials, logs, command execution, and file access. A change that expands the trust boundary needs explicit design discussion.

## Definition of done

- The agreed behavior is implemented or the documentation objective is fulfilled.
- Relevant checks pass and the PR records what ran and what could not run.
- User-facing and developer documentation match the change.
- Compatibility and security implications are explained.
- Review discussions are resolved and the target branch is correct.

Maintainers handle merges, release coordination, and community moderation. No formal election process or service-level response commitment is established. See [Releasing](RELEASING.md) for the release procedure.

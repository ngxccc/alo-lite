---
description: Standardize GitHub issues according to project task templates and labeling conventions
---

Read, analyze, and standardize the GitHub issue: $ARGUMENTS

Workflow:

1. Parse the issue identifier from $ARGUMENTS (URL or #ID).
2. Fetch the current issue details (title, body, labels, state) using project tools.
3. Review and align against:
   - `.github/ISSUE_TEMPLATE/task.yml`
   - `docs/standards/issue-tracker.md`
4. Inspect the relevant codebase to verify technical context and scope (domain, crates, docs, protocol).
5. Check issue atomicity:
   - Assess whether the issue contains more than one distinct problem, feature, or concern.
   - If multiple concerns exist, break them down into separate sub-issues using `gh issue create`, link them back to the original issue, and refine the scope of each.
6. Analyze and standardize:
   - **Title**: Enforce the `<type>(<area>): <summary>` format (Conventional Commits).
   - **Body**: Format according to the task template structure:
     ### Outcome
     ### Context
     ### Acceptance Criteria
     ### Non-goals
   - **Labels**: Apply the appropriate `type:*`, `area:*`, and `priority:*` labels; remove `needs-triage`.
7. Confirm changes and update the issue via GitHub CLI (`gh issue edit`).

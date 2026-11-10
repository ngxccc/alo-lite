export interface CommandApi {
  cwd?: string;
  exec?: (cmd: string, args?: string[], opts?: unknown) => Promise<unknown>;
}

export interface CommandContext {
  cwd?: string;
}

export default function (_api: CommandApi) {
  return {
    name: "standardize-issue",
    description:
      "Standardize GitHub issues according to project task templates and labeling conventions",
    async execute(args: string[], _context: CommandContext, rawArgs: string) {
      const target = rawArgs?.trim() || args.join(" ");
      return `Read, analyze, and standardize the GitHub issue: ${target}

Workflow:
1. Parse the issue identifier from the target (URL https://github.com/username/repo/issues/<id> or #ID).
2. Fetch the current issue details (title, body, labels, state) using project tools or \`gh issue view <id>\`.
3. Review and align against:
   - \`.github/ISSUE_TEMPLATE/task.yml\`
   - \`docs/standards/issue-tracker.md\`
4. Inspect the relevant codebase to verify technical context and scope (crates, modules, UI, protocol, docs).
5. Check issue atomicity:
   - Assess whether the issue contains more than one distinct problem, feature, or concern.
   - If multiple concerns exist, break them down into separate sub-issues using \`gh issue create\`, link them back to the original issue, and refine the scope of each.
6. Analyze and standardize:
   - **Title**: Enforce the \`<type>(<area>): <summary>\` format (Conventional Commits).
   - **Body**: Format according to the task template structure:
     ### Outcome
     ### Context
     ### Acceptance Criteria
     ### Non-goals
   - **Labels**: Apply the appropriate \`type:*\`, \`area:*\`, and \`priority:*\` labels; remove \`needs-triage\`.
7. Confirm changes and update the issue via GitHub CLI (\`gh issue edit\`).`;
    },
  };
}

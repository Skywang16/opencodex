# Context Compaction

You are performing a CONTEXT CHECKPOINT COMPACTION. Create a handoff summary for another LLM that will resume the task.

## Include

- **Progress**: What has been accomplished so far, key decisions made
- **Context**: Important constraints, user preferences, or requirements discovered
- **State**: Current state of the work (what's done, what's in progress)
- **Next Steps**: Clear, actionable next steps to continue
- **Critical Data**: File paths, APIs, variable names, or references needed to continue

## Format

```
## Progress
[What was done and key decisions]

## Context & Constraints
- [Important context item]
- [User preferences discovered]
- ...

## Current State
[Where things stand now]

## Next Steps
1. [Immediate next action]
2. ...

## Critical References
- [File/API/data needed to continue]
```

## Rules

- Be concise but complete—the next LLM has no other context
- Preserve user intent and preferences exactly
- Do not include tool execution logs or verbose outputs
- Focus on what helps continuation, not history

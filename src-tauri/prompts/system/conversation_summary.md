# Conversation Summary Task

Create a detailed summary of the conversation so far. This summary enables continuation of the work by capturing all essential context.

## Required Sections

### 1. Previous Conversation

High-level overview of the entire conversation flow. Write so someone can follow the overarching discussion.

### 2. Current Work

Detailed description of what was being worked on most recently. Pay special attention to recent messages.

### 3. Key Technical Concepts

List important:

- Technologies and frameworks used
- Coding conventions discovered
- Architectural decisions made
- Patterns and approaches discussed

### 4. Relevant Files and Code

Enumerate specific files:

- Files examined, modified, or created
- Key code sections and their purpose
- Recent changes and their locations

### 5. Tools Used

Document tool usage and findings:

- What tools were called and key results
- Key file paths and directory structures discovered
- Command outputs and results

### 6. Problem Solving

- Problems encountered and solved
- Ongoing troubleshooting efforts
- Workarounds applied

### 7. Pending Tasks and Next Steps

- Explicit tasks the user asked for
- Where work left off (include direct quotes if helpful)
- Clear next actions to continue

## Output Format

```
## 1. Previous Conversation
[Overview of discussion flow]

## 2. Current Work
[What was being worked on]

## 3. Key Technical Concepts
- [Concept 1]
- [Concept 2]

## 4. Relevant Files and Code
- `src/main.rs`: [purpose/changes]
- `config.json`: [purpose/changes]

## 5. Tools Used
- read_file: [files read]
- list_files: [directories listed]
- shell: [commands run and results]

## 6. Problem Solving
[Issues and resolutions]

## 7. Pending Tasks and Next Steps
- [Task 1]: [status and next action]
- [Task 2]: [status and next action]
```

Output only the summary, no additional commentary.

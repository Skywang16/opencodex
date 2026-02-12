# Type System Architecture

## Principle: Single Source of Truth

All domain types live in `src/types/domain/`. API modules (`src/api/*/types.ts`)
**re-export** from domain — they never duplicate definitions.

```
src/types/
├── index.ts           # Main entry, re-exports all
├── core/              # Core utility types (OperationResult, BaseConfig, etc.)
│   ├── common.ts
│   └── api.ts
├── domain/            # Domain types — the ONLY place types are defined
│   ├── ai.ts
│   ├── aiMessage.ts
│   ├── completion.ts
│   ├── shortcuts.ts
│   ├── storage.ts     # Includes SessionState, WindowGeometry, etc.
│   ├── terminal.ts    # Includes ShellInfo, CommandStatus, etc.
│   ├── theme.ts
│   └── ...
├── tags/
└── utils/
```

## Rules

1. **Define once**: Every interface/type has exactly one source file in `types/domain/`.
2. **API re-exports**: `src/api/*/types.ts` files only contain `export type { ... } from '@/types/domain/...'`.
3. **Import via barrel**: Components and stores import from `@/types` or `@/api`.
4. **No manual snake_case → camelCase mapping**: Backend uses `#[serde(rename_all = "camelCase")]`.

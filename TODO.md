# Improvements

- [x] **Avoid Orchestrator rewriting context**: Implemented artifact system. Agent responses are stored as numbered artifacts and auto-injected into subsequent agents' system prompts. The Orchestrator references them by ID (e.g., "Review Artifact #1") instead of copying content.
- [x] **Context overflow fix**: Truncate agent responses to 500 chars in orchestrator_messages, budget-based `build_artifact_context` (4000 chars), sliding window (8000 chars) before each Orchestrator call.
- [x] **Pre-inject directory listing**: Working directory contents are pre-listed and injected into agent system prompts so they don't waste tool calls on `list_directory(".")`.
- [x] **Graceful tool loop degradation**: After 3 consecutive tool failures, forces text response. Exceeding max rounds (6) returns a summary instead of killing the orchestration.

# Pending

- [ ] **Agents don't see original user request**: Each agent only sees the Orchestrator's paraphrased instruction. The Architect changed `price: float` to `description: str` because the original spec wasn't passed directly. Fix: inject the task description into each agent's user message alongside the delegation instruction.
- [ ] **Judge doesn't compare against user spec**: The Judge reviews the Architect's plan in isolation without checking it against what the user actually asked for. Fix: update Judge system prompt to mandate comparison against the original request as the first check.
- [ ] **Judge corrections not incorporated before construction**: The Judge identified real issues (fragile ID generation) but the Constructor implemented the original plan without incorporating the Judge's feedback. The Orchestrator should re-delegate to the Architect or Constructor with the Judge's corrections before proceeding.
- [ ] **SLM code quality**: Generated code uses deprecated APIs (`item.dict()` → `model_dump()`, `@app.on_event("startup")` → `lifespan`). Consider adding a "best practices" section to Constructor's system prompt for common frameworks.

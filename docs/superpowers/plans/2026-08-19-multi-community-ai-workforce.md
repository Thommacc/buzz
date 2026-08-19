# Multi-Community AI Workforce Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Represent every AI employee or Hermes identity once in Buzz while safely resolving community membership, approved company context, and model policy from the authenticated relay, with data-only onboarding for Damen and DSRY.

**Architecture:** Keep Buzz's existing `(pubkey, relay_url)` runtime pairs as transport connectors, but make a single managed-agent record the logical identity. Add a versioned workforce store beside `managed-agents.json` for community records, memberships, approved/proposed context, and model overrides. At the shared spawn boundary, resolve the authenticated relay to a company, fail closed when membership or context is invalid, compose the role prompt with only approved company context, and apply task/company/role model precedence. The UI renders the logical identity once and exposes memberships, routing, and an intake role selector without rendering connector pairs as employees.

**Tech Stack:** Rust/Tauri, Serde JSON stores, TypeScript/React, TanStack Query, Node test runner, Rust unit/integration tests, existing Buzz ACP runtime environment contract.

---

## Safety and delivery constraints

- Preserve the five unrelated dirty files already present in the source tree.
- Develop in an isolated Git worktree and use signed-off commits.
- Do not restart Buzz Desktop, WSL, Hermes, or existing agent processes during implementation.
- Do not mutate the live `managed-agents.json` until the migration dry-run, backup, canary, and rollback checks pass.
- Never copy Hermes secrets, prompts, models, or tool configuration into the workforce store.
- Treat canonical authenticated relay URLs as the company boundary; names or task text never select a company.

## Task 1: Create the typed workforce domain and validation rules

**Files:**

- Create: `desktop/src-tauri/src/managed_agents/workforce/types.rs`
- Create: `desktop/src-tauri/src/managed_agents/workforce/validation.rs`
- Create: `desktop/src-tauri/src/managed_agents/workforce/tests.rs`
- Create: `desktop/src-tauri/src/managed_agents/workforce/mod.rs`
- Modify: `desktop/src-tauri/src/managed_agents/mod.rs`

- [x] Write failing tests for schema-version rejection, duplicate company IDs, duplicate canonical relays, unknown role/member references, invalid lifecycle states, unapproved context exclusion, and Hermes reference validation.
- [x] Define `WorkforceStore`, `WorkforceIdentity`, `CommunityRecord`, `CommunityMembership`, `CompanyContext`, `ContextFact`, `ModelPolicy`, and `RolloutState` with backward-compatible Serde defaults.
- [x] Add canonical relay lookup and validation that fails closed on unknown, duplicated, disabled, or malformed communities.
- [x] Keep Hermes identities reference-only: stable pubkey/profile reference plus membership metadata, with no prompt/model/tool fields.
- [x] Run `cargo test --manifest-path desktop/src-tauri/Cargo.toml --lib managed_agents::workforce` from the repository root.
- [x] Commit with `git commit -s -m "feat(workforce): add typed community registry"`.

## Task 2: Add atomic workforce storage and audit-safe context approval

**Files:**

- Create: `desktop/src-tauri/src/managed_agents/workforce/storage.rs`
- Create: `desktop/src-tauri/src/managed_agents/workforce/storage_tests.rs`
- Modify: `desktop/src-tauri/src/managed_agents/workforce/mod.rs`
- Modify: `desktop/src-tauri/src/managed_agents/storage.rs`

- [x] Write failing tests for missing-store defaults, invalid JSON evidence preservation, atomic save, `0o600` permissions on Unix, last-approved-version fallback, proposal approval, rejection, supersession, and monotonic context versions.
- [x] Store control-plane data at `agents/workforce/workforce.json` and context packages at `agents/workforce/contexts/<company_id>.json`; use temp-file plus rename semantics matching the managed-agent store.
- [x] Implement proposal and approval operations that keep source, timestamp, reviewer, confidence, and status, and inject only approved non-superseded facts.
- [x] Ensure failed writes retain the previous approved version and surface a diagnostic.
- [x] Run `cargo test --manifest-path desktop/src-tauri/Cargo.toml --lib managed_agents::workforce`.
- [x] Commit with `git commit -s -m "feat(workforce): persist approved company context"`.

## Task 3: Resolve membership, prompt layers, and model precedence

**Files:**

- Create: `desktop/src-tauri/src/managed_agents/workforce/resolver.rs`
- Create: `desktop/src-tauri/src/managed_agents/workforce/resolver_tests.rs`
- Modify: `desktop/src-tauri/src/managed_agents/runtime.rs`
- Modify: `desktop/src-tauri/src/managed_agents/spawn_hash.rs`
- Modify: `desktop/src-tauri/src/managed_agents/runtime/tests.rs`

- [x] Write failing tests proving that the canonical runtime relay selects the company, an unknown/disabled/non-member relay refuses before process side effects, and company text cannot switch the resolved company.
- [x] Write failing tests for prompt order: immutable guardrails, general role prompt, approved company context, company-local references, then task constraints; verify proposed/rejected facts and secrets are absent.
- [x] Write failing tests for model precedence: explicit task override, company-role override, role default, and fail-closed when no approved healthy route exists.
- [x] Add `ResolvedWorkforceExecution` with company/context/role versions and hashes suitable for receipts without logging prompt text or secrets.
- [x] Call the resolver once in `spawn_agent_child`, then emit the composed prompt/model while retaining current behavior for identities not yet enrolled in workforce mode.
- [x] Include the workforce resolution in `spawn_config_hash` so a context or model-policy change produces an accurate restart-needed signal.
- [x] Run the focused `managed_agents::workforce`, `managed_agents::runtime`, and `managed_agents::spawn_hash` tests with `cargo test --manifest-path desktop/src-tauri/Cargo.toml --lib <filter>`.
- [x] Commit with `git commit -s -m "feat(workforce): resolve community context at spawn"`.

## Task 4: Make community membership control runtime fan-out

**Files:**

- Modify: `desktop/src-tauri/src/managed_agents/runtime_commands.rs`
- Modify: `desktop/src-tauri/src/managed_agents/runtime_types.rs`
- Modify: `desktop/src-tauri/src/managed_agents/restore.rs`
- Modify: `desktop/src-tauri/src/managed_agents/runtime/stop.rs`

- [x] Write failing tests that one logical employee fans out only to enabled memberships, one failed community does not block another, and stopping one pair leaves the identity's other community pair running.
- [x] Filter proactive reconciliation through workforce membership instead of every local auto-start record joining every configured community.
- [x] Preserve explicit/manual pair start for an enabled membership and fail closed for a disabled or absent membership.
- [x] Restore only previously valid enabled pairs; never auto-start draft Damen/DSRY memberships.
- [x] Prove the same pubkey remains the runtime identity across two relay pairs.
- [x] Run the focused runtime and restore tests.
- [x] Commit with `git commit -s -m "feat(workforce): enforce community memberships"`.

## Task 5: Expose workforce commands and TypeScript API contracts

**Files:**

- Create: `desktop/src-tauri/src/commands/workforce.rs`
- Modify: `desktop/src-tauri/src/commands/mod.rs`
- Modify: `desktop/src-tauri/src/lib.rs`
- Create: `desktop/src/shared/api/tauriWorkforce.ts`
- Modify: `desktop/src/shared/api/types.ts`
- Create: `desktop/src/features/agents/workforceHooks.ts`

- [x] Write Rust command tests for list/read, upsert draft community, membership enable/disable, context proposal/approval, company override, and task override validation.
- [x] Expose only redacted summaries to the frontend; no nsecs, connector credentials, or unapproved context bodies in list responses.
- [x] Add TypeScript request/response types and query/mutation hooks with invalidation of agents, runtimes, and workforce summaries.
- [x] Add an optimistic-concurrency version to mutations so stale edits cannot overwrite newer approvals.
- [x] Run the focused Rust tests and `pnpm --dir desktop typecheck`.
- [ ] Commit with `git commit -s -m "feat(workforce): expose community control API"`.

## Task 6: Render one identity and manage its communities

**Files:**

- Modify: `desktop/src/features/agents/ui/unifiedAgentGroups.ts`
- Create: `desktop/src/features/agents/ui/unifiedAgentGroups.test.mjs`
- Modify: `desktop/src/features/agents/ui/UnifiedAgentsSection.tsx`
- Create: `desktop/src/features/agents/ui/AgentCommunityMemberships.tsx`
- Modify: `desktop/src/features/settings/ui/ActiveAgentCommunitiesSettingsCard.tsx`
- Modify: `desktop/src/features/agents/AGENTS.md`

- [x] Write failing UI-core tests that duplicate legacy rows sharing a pubkey collapse to one identity, distinct pubkeys remain separate, and runtime connector rows never become employee cards.
- [x] Group by stable identity/pubkey first and persona second; show one name/avatar/role with community badges and per-community runtime health.
- [x] Add membership controls for existing Buzz employees and a separate Hermes section whose entries are identity references only.
- [x] Show role default model, company override, resolved model, and why that route won.
- [x] Keep draft/canary memberships visibly stopped and require explicit enable/start actions.
- [x] Update the feature-level `AGENTS.md` because config modeling and rendering changed.
- [x] Run `pnpm --dir desktop test -- unifiedAgentGroups`, `pnpm --dir desktop typecheck`, and `pnpm --dir desktop check`.
- [x] Commit with `git commit -s -m "feat(workforce): show one agent across communities"`.

## Task 7: Add intake and role discovery for all 35 employees

**Files:**

- Create: `desktop/src/features/agents/intake/roleIntake.ts`
- Create: `desktop/src/features/agents/intake/roleIntake.test.mjs`
- Create: `desktop/src/features/agents/ui/AgentIntakePanel.tsx`
- Modify: `desktop/src/features/agents/ui/UnifiedAgentsSection.tsx`
- Create: `desktop/src-tauri/resources/workforce/role-catalog.v1.json`

- [x] Import the 35 existing role identities into a data catalog with department, plain-language `useWhen`, required inputs, expected outputs, safety boundary, and default model policy.
- [x] Write deterministic intake tests for representative finance, sales, HR, legal, operations, marketing, and technical tasks, returning one primary and at most two supporting roles.
- [x] Add a searchable directory and `AI Team - Intake & Regie` panel that explains the recommendation, inputs/outputs, resolved model, and approval needs before work starts.
- [x] Ensure intake recommends only roles enabled in the authenticated active community.
- [x] Run intake tests, typecheck, and frontend checks.
- [x] Commit with `git commit -s -m "feat(workforce): add role intake and directory"`.

## Task 8: Build a reversible legacy-store migration tool

**Files:**

- Create: `crates/buzz-workforce-migrate/src/main.rs`
- Create: `crates/buzz-workforce-migrate/tests/migration.rs`
- Create: `docs/runbooks/workforce-migration.md`

- [x] Write fixture-based tests for the current pattern: three records with the same pubkey/profile become one canonical managed-agent identity plus three memberships, with per-company model/context metadata preserved.
- [x] Refuse conflicting keys, prompts, avatars, persona IDs, or unsupported records and report them without writing.
- [x] Implement default `--dry-run`; require explicit `--apply`, write timestamped backups and a machine-readable receipt, and support receipt-driven rollback.
- [x] Make reruns idempotent and keep running-process receipts untouched.
- [x] Document exact preflight, backup, canary, apply, verify, and rollback commands.
- [x] Run migration fixtures twice to prove idempotence and rollback byte equality.
- [x] Commit with `git commit -s -m "feat(workforce): add reversible identity migration"`.

## Task 9: Seed existing companies, Damen, DSRY, and Hermes references

**Files:**

- Create: `desktop/src-tauri/resources/workforce/community-registry.v1.json`
- Create: `desktop/src-tauri/resources/workforce/contexts/thommacclabs.v1.json`
- Create: `desktop/src-tauri/resources/workforce/contexts/totaltools.v1.json`
- Create: `desktop/src-tauri/resources/workforce/contexts/vanderhilst.v1.json`
- Create: `desktop/src-tauri/resources/workforce/contexts/damen.v1.json`
- Create: `desktop/src-tauri/resources/workforce/contexts/dsry.v1.json`
- Create: `desktop/src-tauri/resources/workforce/hermes-identities.v1.json`
- Create: `desktop/src-tauri/tests/workforce_seed_validation.rs`

- [x] Generate existing-company records from the read-only live inventory, not remembered relay IDs.
- [x] Seed Damen and DSRY as `draft` with no relay URL guessed, no autostart, and only sourced/approved facts; unknown fields remain explicit review items.
- [x] Import current Hermes pubkeys/profile references and observed memberships without copying prompts, models, tools, keys, or starting processes.
- [x] Validate every seed against the typed schema and prove no duplicated pubkey is rendered as a second identity.
- [x] Run seed validation and full focused backend/frontend tests.
- [x] Commit with `git commit -s -m "data(workforce): seed companies and Hermes identities"`.

## Task 10: Verify, canary, and stage the live rollout

**Files:**

- Create: `docs/runbooks/workforce-canary-checklist.md`
- Update: `/home/thoma/SYSTEM_LOG.md` only when a cross-system live canary or migration actually changes state.
- Update: the relevant Hermes `AGENT_LOG.md` only if a Hermes runtime/config path is actually changed.

- [x] Run `cargo fmt --manifest-path desktop/src-tauri/Cargo.toml --all -- --check` and the focused Rust suite, then `cargo test --manifest-path desktop/src-tauri/Cargo.toml`.
- [x] Run `pnpm --dir desktop test`, `pnpm --dir desktop typecheck`, `pnpm --dir desktop check`, and the relevant Playwright smoke flow. The workforce-relevant flows pass; one unchanged memory-sharing layout assertion is recorded in the canary checklist.
- [x] Build the desktop artifact without launching it and record artifact hashes.
- [x] Run the migration tool against a copied live store in dry-run mode; verify 35 logical employees, all expected memberships, zero key conflicts, and byte-preserving rollback.
- [ ] Capture a fresh live backup and process/receipt inventory before any canary.
- [ ] Canary one low-risk employee on one existing community, then the same pubkey on a second community; prove prompt/context isolation and model precedence.
- [ ] Canary one existing Hermes identity in an additional community without changing or restarting its base Hermes runtime.
- [ ] Obtain exact owned relay records for Damen and DSRY before moving either from `draft`; activate one at a time with other memberships stopped.
- [ ] Update the system journal with scope, reason, backup, impact, rollback, and verified checks for any live state change.
- [ ] Present canary evidence and stop for Thom's acceptance before broad migration or autostart.

## Completion criteria

- [x] The Buzz directory displays 35 logical AI employees once each and every registered Hermes identity once.
- [x] Each identity can list multiple community memberships without cloned profiles.
- [x] Runtime context is selected only from the authenticated canonical relay and cross-company probes fail closed.
- [x] Approved facts are injected; proposed/rejected facts are not.
- [x] Task override > company-role override > role default is tested and visible.
- [x] Adding a draft company or membership is data-only.
- [x] Damen and DSRY have validated draft context packages and remain stopped until exact relay ownership and canaries pass.
- [x] The legacy store migration is dry-run-first, backed up, idempotent, and rollback verified.
- [x] Existing WSL, Hermes, Buzz, and healthy agent processes were not disrupted by staging.

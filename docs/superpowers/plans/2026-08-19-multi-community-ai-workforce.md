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

- [ ] Write failing tests for schema-version rejection, duplicate company IDs, duplicate canonical relays, unknown role/member references, invalid lifecycle states, unapproved context exclusion, and Hermes reference validation.
- [ ] Define `WorkforceStore`, `WorkforceIdentity`, `CommunityRecord`, `CommunityMembership`, `CompanyContext`, `ContextFact`, `ModelPolicy`, and `RolloutState` with backward-compatible Serde defaults.
- [ ] Add canonical relay lookup and validation that fails closed on unknown, duplicated, disabled, or malformed communities.
- [ ] Keep Hermes identities reference-only: stable pubkey/profile reference plus membership metadata, with no prompt/model/tool fields.
- [ ] Run `cargo test -p buzz-desktop managed_agents::workforce` from the repository root.
- [ ] Commit with `git commit -s -m "feat(workforce): add typed community registry"`.

## Task 2: Add atomic workforce storage and audit-safe context approval

**Files:**

- Create: `desktop/src-tauri/src/managed_agents/workforce/storage.rs`
- Create: `desktop/src-tauri/src/managed_agents/workforce/storage_tests.rs`
- Modify: `desktop/src-tauri/src/managed_agents/workforce/mod.rs`
- Modify: `desktop/src-tauri/src/managed_agents/storage.rs`

- [ ] Write failing tests for missing-store defaults, invalid JSON evidence preservation, atomic save, `0o600` permissions on Unix, last-approved-version fallback, proposal approval, rejection, supersession, and monotonic context versions.
- [ ] Store control-plane data at `agents/workforce/workforce.json` and context packages at `agents/workforce/contexts/<company_id>.json`; use temp-file plus rename semantics matching the managed-agent store.
- [ ] Implement proposal and approval operations that keep source, timestamp, reviewer, confidence, and status, and inject only approved non-superseded facts.
- [ ] Ensure failed writes retain the previous approved version and surface a diagnostic.
- [ ] Run `cargo test -p buzz-desktop managed_agents::workforce`.
- [ ] Commit with `git commit -s -m "feat(workforce): persist approved company context"`.

## Task 3: Resolve membership, prompt layers, and model precedence

**Files:**

- Create: `desktop/src-tauri/src/managed_agents/workforce/resolver.rs`
- Create: `desktop/src-tauri/src/managed_agents/workforce/resolver_tests.rs`
- Modify: `desktop/src-tauri/src/managed_agents/runtime.rs`
- Modify: `desktop/src-tauri/src/managed_agents/spawn_hash.rs`
- Modify: `desktop/src-tauri/src/managed_agents/runtime/tests.rs`

- [ ] Write failing tests proving that the canonical runtime relay selects the company, an unknown/disabled/non-member relay refuses before process side effects, and company text cannot switch the resolved company.
- [ ] Write failing tests for prompt order: immutable guardrails, general role prompt, approved company context, company-local references, then task constraints; verify proposed/rejected facts and secrets are absent.
- [ ] Write failing tests for model precedence: explicit task override, company-role override, role default, and fail-closed when no approved healthy route exists.
- [ ] Add `ResolvedWorkforceExecution` with company/context/role versions and hashes suitable for receipts without logging prompt text or secrets.
- [ ] Call the resolver once in `spawn_agent_child`, then emit the composed prompt/model while retaining current behavior for identities not yet enrolled in workforce mode.
- [ ] Include the workforce resolution in `spawn_config_hash` so a context or model-policy change produces an accurate restart-needed signal.
- [ ] Run `cargo test -p buzz-desktop managed_agents::workforce managed_agents::runtime managed_agents::spawn_hash`.
- [ ] Commit with `git commit -s -m "feat(workforce): resolve community context at spawn"`.

## Task 4: Make community membership control runtime fan-out

**Files:**

- Modify: `desktop/src-tauri/src/managed_agents/runtime_commands.rs`
- Modify: `desktop/src-tauri/src/managed_agents/runtime_types.rs`
- Modify: `desktop/src-tauri/src/managed_agents/restore.rs`
- Modify: `desktop/src-tauri/src/managed_agents/runtime/stop.rs`

- [ ] Write failing tests that one logical employee fans out only to enabled memberships, one failed community does not block another, and stopping one pair leaves the identity's other community pair running.
- [ ] Filter proactive reconciliation through workforce membership instead of every local auto-start record joining every configured community.
- [ ] Preserve explicit/manual pair start for an enabled membership and fail closed for a disabled or absent membership.
- [ ] Restore only previously valid enabled pairs; never auto-start draft Damen/DSRY memberships.
- [ ] Prove the same pubkey remains the runtime identity across two relay pairs.
- [ ] Run the focused runtime and restore tests.
- [ ] Commit with `git commit -s -m "feat(workforce): enforce community memberships"`.

## Task 5: Expose workforce commands and TypeScript API contracts

**Files:**

- Create: `desktop/src-tauri/src/commands/workforce.rs`
- Modify: `desktop/src-tauri/src/commands/mod.rs`
- Modify: `desktop/src-tauri/src/lib.rs`
- Create: `desktop/src/shared/api/tauriWorkforce.ts`
- Modify: `desktop/src/shared/api/types.ts`
- Create: `desktop/src/features/agents/workforceHooks.ts`

- [ ] Write Rust command tests for list/read, upsert draft community, membership enable/disable, context proposal/approval, company override, and task override validation.
- [ ] Expose only redacted summaries to the frontend; no nsecs, connector credentials, or unapproved context bodies in list responses.
- [ ] Add TypeScript request/response types and query/mutation hooks with invalidation of agents, runtimes, and workforce summaries.
- [ ] Add an optimistic-concurrency version to mutations so stale edits cannot overwrite newer approvals.
- [ ] Run the focused Rust tests and `pnpm --dir desktop typecheck`.
- [ ] Commit with `git commit -s -m "feat(workforce): expose community control API"`.

## Task 6: Render one identity and manage its communities

**Files:**

- Modify: `desktop/src/features/agents/ui/unifiedAgentGroups.ts`
- Create: `desktop/src/features/agents/ui/unifiedAgentGroups.test.mjs`
- Modify: `desktop/src/features/agents/ui/UnifiedAgentsSection.tsx`
- Create: `desktop/src/features/agents/ui/AgentCommunityMemberships.tsx`
- Modify: `desktop/src/features/settings/ui/ActiveAgentCommunitiesSettingsCard.tsx`
- Modify: `desktop/src/features/agents/AGENTS.md`

- [ ] Write failing UI-core tests that duplicate legacy rows sharing a pubkey collapse to one identity, distinct pubkeys remain separate, and runtime connector rows never become employee cards.
- [ ] Group by stable identity/pubkey first and persona second; show one name/avatar/role with community badges and per-community runtime health.
- [ ] Add membership controls for existing Buzz employees and a separate Hermes section whose entries are identity references only.
- [ ] Show role default model, company override, resolved model, and why that route won.
- [ ] Keep draft/canary memberships visibly stopped and require explicit enable/start actions.
- [ ] Update the feature-level `AGENTS.md` because config modeling and rendering changed.
- [ ] Run `pnpm --dir desktop test -- unifiedAgentGroups`, `pnpm --dir desktop typecheck`, and `pnpm --dir desktop check`.
- [ ] Commit with `git commit -s -m "feat(workforce): show one agent across communities"`.

## Task 7: Add intake and role discovery for all 35 employees

**Files:**

- Create: `desktop/src/features/agents/intake/roleIntake.ts`
- Create: `desktop/src/features/agents/intake/roleIntake.test.mjs`
- Create: `desktop/src/features/agents/ui/AgentIntakePanel.tsx`
- Modify: `desktop/src/features/agents/ui/UnifiedAgentsSection.tsx`
- Create: `desktop/src-tauri/resources/workforce/role-catalog.v1.json`

- [ ] Import the 35 existing role identities into a data catalog with department, plain-language `useWhen`, required inputs, expected outputs, safety boundary, and default model policy.
- [ ] Write deterministic intake tests for representative finance, sales, HR, legal, operations, marketing, and technical tasks, returning one primary and at most two supporting roles.
- [ ] Add a searchable directory and `AI Team - Intake & Regie` panel that explains the recommendation, inputs/outputs, resolved model, and approval needs before work starts.
- [ ] Ensure intake recommends only roles enabled in the authenticated active community.
- [ ] Run intake tests, typecheck, and frontend checks.
- [ ] Commit with `git commit -s -m "feat(workforce): add role intake and directory"`.

## Task 8: Build a reversible legacy-store migration tool

**Files:**

- Create: `desktop/src-tauri/src/bin/buzz-workforce-migrate.rs`
- Create: `desktop/src-tauri/tests/workforce_migration.rs`
- Create: `docs/runbooks/workforce-migration.md`

- [ ] Write fixture-based tests for the current pattern: three records with the same pubkey/profile become one canonical managed-agent identity plus three memberships, with per-company model/context metadata preserved.
- [ ] Refuse conflicting keys, prompts, avatars, persona IDs, or unsupported records and report them without writing.
- [ ] Implement default `--dry-run`; require explicit `--apply`, write timestamped backups and a machine-readable receipt, and support receipt-driven rollback.
- [ ] Make reruns idempotent and keep running-process receipts untouched.
- [ ] Document exact preflight, backup, canary, apply, verify, and rollback commands.
- [ ] Run migration fixtures twice to prove idempotence and rollback byte equality.
- [ ] Commit with `git commit -s -m "feat(workforce): add reversible identity migration"`.

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

- [ ] Generate existing-company records from the read-only live inventory, not remembered relay IDs.
- [ ] Seed Damen and DSRY as `draft` with no relay URL guessed, no autostart, and only sourced/approved facts; unknown fields remain explicit review items.
- [ ] Import current Hermes pubkeys/profile references and observed memberships without copying prompts, models, tools, keys, or starting processes.
- [ ] Validate every seed against the typed schema and prove no duplicated pubkey is rendered as a second identity.
- [ ] Run seed validation and full focused backend/frontend tests.
- [ ] Commit with `git commit -s -m "data(workforce): seed companies and Hermes identities"`.

## Task 10: Verify, canary, and stage the live rollout

**Files:**

- Create: `docs/runbooks/workforce-canary-checklist.md`
- Update: `/home/thoma/SYSTEM_LOG.md` only when a cross-system live canary or migration actually changes state.
- Update: the relevant Hermes `AGENT_LOG.md` only if a Hermes runtime/config path is actually changed.

- [ ] Run `cargo fmt --all -- --check` and the focused Rust suite, then `cargo test -p buzz-desktop`.
- [ ] Run `pnpm --dir desktop test`, `pnpm --dir desktop typecheck`, `pnpm --dir desktop check`, and the relevant Playwright smoke flow.
- [ ] Build the desktop artifact without launching it and record artifact hashes.
- [ ] Run the migration tool against a copied live store in dry-run mode; verify 35 logical employees, all expected memberships, zero key conflicts, and byte-preserving rollback.
- [ ] Capture a fresh live backup and process/receipt inventory before any canary.
- [ ] Canary one low-risk employee on one existing community, then the same pubkey on a second community; prove prompt/context isolation and model precedence.
- [ ] Canary one existing Hermes identity in an additional community without changing or restarting its base Hermes runtime.
- [ ] Obtain exact owned relay records for Damen and DSRY before moving either from `draft`; activate one at a time with other memberships stopped.
- [ ] Update the system journal with scope, reason, backup, impact, rollback, and verified checks for any live state change.
- [ ] Present canary evidence and stop for Thom's acceptance before broad migration or autostart.

## Completion criteria

- [ ] The Buzz directory displays 35 logical AI employees once each and every registered Hermes identity once.
- [ ] Each identity can list multiple community memberships without cloned profiles.
- [ ] Runtime context is selected only from the authenticated canonical relay and cross-company probes fail closed.
- [ ] Approved facts are injected; proposed/rejected facts are not.
- [ ] Task override > company-role override > role default is tested and visible.
- [ ] Adding a draft company or membership is data-only.
- [ ] Damen and DSRY have validated draft context packages and remain stopped until exact relay ownership and canaries pass.
- [ ] The legacy store migration is dry-run-first, backed up, idempotent, and rollback verified.
- [ ] Existing WSL, Hermes, Buzz, and healthy agent processes were not disrupted by staging.

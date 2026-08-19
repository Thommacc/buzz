# Workforce canary checklist

Date: 2026-08-19

## Staging evidence

- Branch: `feature/multi-community-workforce`
- Full Rust suite: 1,823 passed, 14 ignored, 0 failed; all migration and seed integration tests passed.
- Frontend suite: 3,650 passed, 0 failed; typecheck, repository checks, and production build passed.
- Playwright: 7 edit/model tests and 18 of 19 Agents integration tests passed. The workforce-relevant duplicate-instance test passed. One unchanged memory-sharing layout assertion remains reproducibly failing because its right-edge position differs by about 35-39 px.
- Copied-live-store dry-run: 35 identities, 70 duplicate records removable, 105 memberships, 0 conflicts.
- Copied-live-store apply and second run: 35 identities, 3 active communities, 105 memberships; second run reported no changes.
- Copied-live-store rollback: byte-exact (`feee7374df089daf0eea8b38a04ce7ae4eca35db77d0a61929f8ef99dfeb111d` before and after).
- Release binary: `101407c62017f96c9a9affc7a9f0160add8ec637b8ddc8995151b92ed09e006b` (91,874,088 bytes).
- Production frontend index: `e1396905ea122c299b801c2fa44b163b0500f61efbc16b949e9f9290d2d3360d`.
- Production frontend file-set digest: `eb15baeb17b791d87a00f3590e5365507560f1282e1bcc0716fc0c1ca2e11566`.

No live Buzz store, relay, agent process, WSL service, or Hermes runtime/configuration was changed during staging.

## Approval gate

Do not run a live migration or canary until Thom explicitly approves it. Before any approved change:

- [ ] Re-read the live managed-agent store and confirm no target employee has a running `runtime_pid`.
- [ ] Record active Buzz/Hermes processes without stopping or restarting them.
- [ ] Copy the exact live managed-agent and workforce files to timestamped backups.
- [ ] Run the migrator in dry-run mode against the current live files and require 0 conflicts.
- [ ] Record the dry-run output, source hashes, intended canary identity, community, and rollback receipt path.

## Employee canary

- [ ] Apply the reversible migration with an explicit receipt path.
- [ ] Enable one low-risk employee in one existing community; leave all other new memberships stopped.
- [ ] Verify the authenticated canonical relay selects only that company's approved context.
- [ ] Verify proposed/rejected facts are absent from the runtime prompt.
- [ ] Verify model precedence is task override, then company-role override, then role default.
- [ ] Add the same pubkey to a second existing community and prove the identity remains single while context stays isolated.
- [ ] Stop for Thom's acceptance before enabling additional employees or autostart.

## Hermes reference canary

- [ ] Select one existing Hermes identity and add only its public reference to one additional community.
- [ ] Verify no Hermes prompt, model, tool, key, environment, service unit, or process was copied or changed.
- [ ] Verify the base Hermes runtime was not restarted.
- [ ] Stop for Thom's acceptance.

## Damen and DSRY

- [ ] Obtain and verify the exact owned canonical relay for Damen.
- [ ] Obtain and verify the exact owned canonical relay for DSRY.
- [ ] Review and approve each company's context facts; keep unknowns as review items.
- [ ] Activate only one draft community at a time, with all memberships initially stopped.
- [ ] Complete the same employee and Hermes isolation checks before enabling the other community.

## Rollback

- [ ] Stop only the newly enabled Buzz canary membership; do not stop WSL or a base Hermes runtime.
- [ ] Run receipt-driven rollback only if the migrated files still match the receipt's post-migration hashes.
- [ ] Verify the managed-agent file matches its pre-migration SHA-256 and that the prior workforce file is restored or removed as recorded.
- [ ] Record scope, reason, impact, backup, rollback, and checks in `/home/thoma/SYSTEM_LOG.md`; update a Hermes `AGENT_LOG.md` only if a Hermes runtime/config path was actually changed.

# Workforce identity migration

This migration collapses the proven legacy pattern of one employee record per
company into one managed-agent record plus company memberships. It is offline
store maintenance: stop the selected Buzz employee processes first. It never
starts, stops, or restarts Buzz, WSL, or Hermes itself.

## Preconditions

- Build from the reviewed commit and use absolute paths.
- Export a fresh company registry from the current Buzz community inventory.
- Confirm each target persona has one pubkey, prompt, avatar, and name.
- Confirm every target record has `runtime_pid: null`.
- Keep the 35-role catalog and company registry under review; do not guess
  relay URLs or mark an unverified model route healthy.

```bash
cd /home/thoma/buzz
. ./bin/activate-hermit
cargo build -p buzz-workforce-migrate
```

Set explicit paths for the copied stores used in rehearsal:

```bash
MIGRATOR=target/debug/buzz-workforce-migrate
MANAGED=/absolute/rehearsal/managed-agents.json
WORKFORCE=/absolute/rehearsal/workforce.json
CATALOG=desktop/src-tauri/resources/workforce/role-catalog.v1.json
COMPANIES=/absolute/rehearsal/community-registry.json
RECEIPT=/absolute/rehearsal/migration-receipt.json
```

## Dry-run and verify

Dry-run is the default and performs no writes:

```bash
"$MIGRATOR" \
  --managed-store "$MANAGED" \
  --workforce-store "$WORKFORCE" \
  --catalog "$CATALOG" \
  --company-registry "$COMPANIES" \
  --receipt "$RECEIPT" \
  --dry-run
```

Review `conflicts`, identity count, duplicate-removal count, membership count,
and `changed`. Any conflicting pubkey, persona, prompt, avatar, name, unknown
relay, or running-process receipt refuses the migration before a write.

## Apply to a copy

`--apply` is required for writes. Timestamped byte backups are created before
the two stores are atomically replaced. The JSON receipt records paths and
before/after SHA-256 hashes.

```bash
"$MIGRATOR" \
  --managed-store "$MANAGED" \
  --workforce-store "$WORKFORCE" \
  --catalog "$CATALOG" \
  --company-registry "$COMPANIES" \
  --receipt "$RECEIPT" \
  --apply
```

Run the exact command again with another receipt path. A correct rerun reports
`changed: false`, writes no receipt, and leaves both file hashes unchanged.

Verify on the copy:

```bash
jq '.identities | length' "$WORKFORCE"
jq '[.communities[].memberships[]] | length' "$WORKFORCE"
jq 'group_by(.pubkey) | map(select(length > 1)) | length' "$MANAGED"
```

Expected broad-migration gate: 35 employee identities, the reviewed membership
count, and no duplicate employee pubkeys. Model routes remain unstartable until
their approval and live health fields pass validation.

## Rollback

Rollback refuses if either migrated file changed after the receipt was made.
When hashes match, it restores the managed store byte-for-byte and restores the
previous workforce store or removes the newly created one.

```bash
"$MIGRATOR" --rollback "$RECEIPT"
```

## Live canary

Only after copied-store verification:

1. capture fresh live store hashes and process/runtime receipts;
2. stop one low-risk employee pair through normal Buzz controls;
3. dry-run and apply only the reviewed canary copy/target;
4. start that employee in one existing company, then the same pubkey in a
   second company;
5. verify company context isolation and model precedence;
6. add one Hermes reference to another company without editing or restarting
   its base Hermes runtime;
7. record backup, receipt, result, and rollback in the system journal;
8. stop and obtain Thom's acceptance before broad migration or autostart.

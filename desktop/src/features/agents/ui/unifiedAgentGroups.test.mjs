import assert from "node:assert/strict";
import test from "node:test";

import {
  buildUnifiedGroups,
  dedupeManagedAgentsByPubkey,
} from "./unifiedAgentGroups.ts";
import { partitionWorkforceIdentities } from "./workforcePresentation.ts";

const NONE_ARCHIVED = () => false;

function agent(pubkeyOrOverrides = {}, overrides = {}) {
  const resolvedOverrides =
    typeof pubkeyOrOverrides === "string" ? overrides : pubkeyOrOverrides;
  const pubkey =
    typeof pubkeyOrOverrides === "string"
      ? pubkeyOrOverrides
      : (resolvedOverrides.pubkey ?? "a".repeat(64));
  return {
    pubkey,
    name: resolvedOverrides.name ?? "Agent",
    personaId: resolvedOverrides.personaId ?? null,
    status: resolvedOverrides.status ?? "stopped",
    pid: resolvedOverrides.pid ?? null,
    ...resolvedOverrides,
  };
}

function persona(overrides = {}) {
  return { id: "persona-1", displayName: "Persona", ...overrides };
}

test("archived standalone custom agents are omitted while live peers remain", () => {
  const archived = agent({ pubkey: "a".repeat(64), personaId: null });
  const live = agent({ pubkey: "b".repeat(64), personaId: null });
  const isArchived = (pubkey) => pubkey === archived.pubkey;

  const { ungrouped } = buildUnifiedGroups([], [archived, live], isArchived);

  assert.deepEqual(
    ungrouped.map((agent) => agent.pubkey),
    [live.pubkey],
  );
});

test("archived unknown-persona agents are omitted while live peers remain", () => {
  const archived = agent({ pubkey: "a".repeat(64), personaId: "orphan" });
  const live = agent({ pubkey: "b".repeat(64), personaId: "orphan" });
  const isArchived = (pubkey) => pubkey === archived.pubkey;

  // No persona matches "orphan", so both land in the unknown bucket.
  const { unknown } = buildUnifiedGroups([], [archived, live], isArchived);

  assert.deepEqual(
    unknown.map((agent) => agent.pubkey),
    [live.pubkey],
  );
});

test("matched persona groups keep their full instance list including archived", () => {
  const archived = agent({ pubkey: "a".repeat(64), personaId: "persona-1" });
  const live = agent({ pubkey: "b".repeat(64), personaId: "persona-1" });
  const isArchived = (pubkey) => pubkey === archived.pubkey;

  // The card resolves its own target via pickProfileAgent; the group keeps the
  // archived record so an all-archived persona still forms a card in
  // persona-only mode rather than vanishing from the library.
  const { groups } = buildUnifiedGroups(
    [persona()],
    [archived, live],
    isArchived,
  );

  assert.equal(groups.length, 1);
  assert.deepEqual(
    groups[0].agents.map((agent) => agent.pubkey).sort(),
    [archived.pubkey, live.pubkey].sort(),
  );
});

test("a fail-open predicate keeps every standalone agent discoverable", () => {
  const first = agent({ pubkey: "a".repeat(64), personaId: null });
  const second = agent({ pubkey: "b".repeat(64), personaId: null });

  const { ungrouped } = buildUnifiedGroups([], [first, second], NONE_ARCHIVED);

  assert.equal(ungrouped.length, 2);
});

test("duplicate legacy rows sharing a pubkey collapse to one identity", () => {
  const duplicate = dedupeManagedAgentsByPubkey([
    agent("AA", { name: "Finance", relayUrl: "wss://one" }),
    agent("aa", {
      name: "Finance duplicate",
      relayUrl: "wss://two",
      status: "running",
      pid: 42,
    }),
  ]);

  assert.equal(duplicate.length, 1);
  assert.equal(duplicate[0].name, "Finance duplicate");
});

test("distinct pubkeys remain separate identities", () => {
  assert.equal(
    dedupeManagedAgentsByPubkey([agent("aa"), agent("bb")]).length,
    2,
  );
});

test("one deduplicated identity appears in only one persona group", () => {
  const personas = [
    { id: "finance", displayName: "Finance" },
    { id: "sales", displayName: "Sales" },
  ];
  const result = buildUnifiedGroups(personas, [
    agent("aa", { personaId: "finance", status: "stopped" }),
    agent("AA", { personaId: "finance", status: "running", pid: 7 }),
  ]);

  assert.equal(result.groups[0].agents.length, 1);
  assert.equal(result.groups[1].agents.length, 0);
});

test("Hermes runtime references never become employee cards", () => {
  const result = partitionWorkforceIdentities([
    { identityId: "faye", pubkey: "aa", kind: "employee" },
    {
      identityId: "hermes-atlas",
      pubkey: "bb",
      kind: "hermes",
      hermesProfileRef: "atlas",
    },
  ]);

  assert.deepEqual(
    result.employees.map((identity) => identity.identityId),
    ["faye"],
  );
  assert.deepEqual(
    result.hermes.map((identity) => identity.identityId),
    ["hermes-atlas"],
  );
});

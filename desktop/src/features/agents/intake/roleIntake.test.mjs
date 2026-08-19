import assert from "node:assert/strict";
import test from "node:test";

import { recommendRoles, roleCatalog } from "./roleIntake.ts";

function summary(
  enabledIdentityIds = roleCatalog.map((role) => role.identityId),
) {
  return {
    schemaVersion: 1,
    revision: 1,
    identities: [],
    communities: [
      {
        companyId: "acme",
        displayName: "Acme",
        contextRef: "contexts/acme.json",
        defaultLanguage: "en",
        owners: [],
        approvers: [],
        rolloutState: "active",
        modelPolicy: {},
        memberships: enabledIdentityIds.map((identityId) => ({
          identityId,
          enabled: true,
          startOnAppLaunch: false,
        })),
      },
    ],
  };
}

test("catalog contains the 35 observed employee roles exactly once", () => {
  assert.equal(roleCatalog.length, 35);
  assert.equal(new Set(roleCatalog.map((role) => role.identityId)).size, 35);
});

for (const [area, task, expected] of [
  ["finance", "Check this supplier invoice and billing evidence", "billing"],
  [
    "sales",
    "Prepare prospecting for our ideal customer profile",
    "prospecting",
  ],
  [
    "HR",
    "Make an employee onboarding plan for a new hire",
    "employee-onboarding",
  ],
  ["legal", "Draft a contract clause for legal review", "legal-drafting"],
  ["operations", "Create a handover with owners and open risks", "handover"],
  [
    "marketing",
    "Write a customer case study from this interview",
    "case-study",
  ],
  [
    "technical",
    "Define the API integration technical scope",
    "technical-scoping",
  ],
]) {
  test(`routes a representative ${area} task deterministically`, () => {
    const result = recommendRoles(task, summary(), "acme");
    assert.equal(result?.primary.roleId, expected);
    assert.ok((result?.supporting.length ?? 0) <= 2);
  });
}

test("recommends only roles enabled in the authenticated active community", () => {
  const result = recommendRoles(
    "Draft a contract and technical scope",
    summary(["skye-scoping"]),
    "acme",
  );
  assert.equal(result?.primary.identityId, "skye-scoping");
  assert.deepEqual(result?.supporting, []);
  assert.equal(recommendRoles("contract", summary(), "unknown"), null);
});

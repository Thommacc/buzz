import type {
  WorkforceCommunity,
  WorkforceIdentitySummary,
  WorkforceModelRoute,
} from "@/shared/api/workforceTypes";

export function partitionWorkforceIdentities(
  identities: WorkforceIdentitySummary[],
) {
  return {
    employees: identities.filter((identity) => identity.kind === "employee"),
    hermes: identities.filter((identity) => identity.kind === "hermes"),
  };
}

export function resolveVisibleWorkforceModel(
  identity: WorkforceIdentitySummary,
  community: WorkforceCommunity,
): {
  route: WorkforceModelRoute;
  reason: "company override" | "role default";
} | null {
  if (!identity.role) return null;
  const companyOverride =
    community.modelPolicy.roleOverrides?.[identity.role.roleId];
  return companyOverride
    ? { route: companyOverride, reason: "company override" }
    : { route: identity.role.defaultModel, reason: "role default" };
}

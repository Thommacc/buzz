import roleCatalogJson from "../../../../src-tauri/resources/workforce/role-catalog.v1.json";
import type { WorkforceSummary } from "@/shared/api/workforceTypes";

export type RoleCatalogEntry = (typeof roleCatalogJson.roles)[number];
export const roleCatalog = roleCatalogJson.roles as RoleCatalogEntry[];

export type RoleRecommendation = {
  primary: RoleCatalogEntry;
  supporting: RoleCatalogEntry[];
  reasons: Record<string, string>;
};

export function recommendRoles(
  task: string,
  summary: WorkforceSummary,
  companyId: string,
): RoleRecommendation | null {
  const community = summary.communities.find(
    (item) =>
      item.companyId === companyId &&
      (item.rolloutState === "active" || item.rolloutState === "canary"),
  );
  if (!community) return null;
  const enabled = new Set(
    community.memberships
      .filter((membership) => membership.enabled)
      .map((membership) => membership.identityId),
  );
  const normalized = task.toLocaleLowerCase();
  const ranked = roleCatalog
    .filter((role) => enabled.has(role.identityId))
    .map((role) => ({ role, score: scoreRole(normalized, role) }))
    .filter((candidate) => candidate.score > 0)
    .sort(
      (left, right) =>
        right.score - left.score ||
        left.role.displayName.localeCompare(right.role.displayName),
    );
  const [primary, ...supporting] = ranked;
  if (!primary) return null;
  const selected = [primary, ...supporting.slice(0, 2)];
  return {
    primary: primary.role,
    supporting: selected.slice(1).map((candidate) => candidate.role),
    reasons: Object.fromEntries(
      selected.map(({ role }) => [
        role.identityId,
        `Matches ${matchedKeywords(normalized, role).join(", ")}. ${role.useWhen}`,
      ]),
    ),
  };
}

function scoreRole(task: string, role: RoleCatalogEntry): number {
  return matchedKeywords(task, role).reduce(
    (score, keyword) => score + (keyword.includes(" ") ? 4 : 2),
    task.includes(role.department.toLocaleLowerCase()) ? 1 : 0,
  );
}

function matchedKeywords(task: string, role: RoleCatalogEntry): string[] {
  return role.keywords.filter((keyword) =>
    task.includes(keyword.toLocaleLowerCase()),
  );
}

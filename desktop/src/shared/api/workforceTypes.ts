export type WorkforceModelRouteHealth = "healthy" | "unhealthy" | "unknown";

export type WorkforceModelRoute = {
  provider: string;
  model: string;
  purposeLabel: string;
  approved: boolean;
  health: WorkforceModelRouteHealth;
  lastHealthCheckAt: string;
};

export type WorkforceRoleSummary = {
  roleId: string;
  department: string;
  useWhen: string;
  requiredInputs: string[];
  expectedOutputs: string[];
  defaultModel: WorkforceModelRoute;
  approvalBoundary: string;
  catalogVersion: number;
  lastVerifiedAt: string;
};

export type WorkforceIdentitySummary = {
  identityId: string;
  pubkey: string;
  kind: "employee" | "hermes";
  role?: WorkforceRoleSummary;
  hermesProfileRef?: string;
};

export type WorkforceCommunityMembership = {
  identityId: string;
  enabled: boolean;
  startOnAppLaunch: boolean;
};

export type WorkforceCommunity = {
  companyId: string;
  displayName: string;
  relayUrl?: string;
  contextRef: string;
  defaultLanguage: string;
  owners: string[];
  approvers: string[];
  rolloutState: "draft" | "canary" | "active" | "disabled";
  modelPolicy: { roleOverrides?: Record<string, WorkforceModelRoute> };
  memberships: WorkforceCommunityMembership[];
};

export type WorkforceSummary = {
  schemaVersion: number;
  revision: number;
  identities: WorkforceIdentitySummary[];
  communities: WorkforceCommunity[];
};

export type WorkforceCommunityDraftInput = {
  companyId: string;
  displayName: string;
  defaultLanguage: string;
  owners?: string[];
  approvers?: string[];
};

export type WorkforceContextFactStatus =
  | { status: "proposed" }
  | { status: "approved"; reviewer: string; reviewed_at: string }
  | {
      status: "rejected";
      reviewer: string;
      reviewed_at: string;
      reason: string;
    }
  | {
      status: "superseded";
      replacement_fact_id: string;
      superseded_at: string;
    };

export type WorkforceContextFact = {
  factId: string;
  statement: string;
  source: string;
  proposedAt: string;
  confidenceBasisPoints: number;
  status: WorkforceContextFactStatus;
};

export type WorkforceContextReviewDecision =
  | { decision: "approve" }
  | { decision: "reject"; reason: string };

export type WorkforceRoutePreview = {
  companyId: string;
  identityId: string;
  roleId?: string;
  workforceRevision: number;
  contextVersion: number;
  model?: WorkforceModelRoute;
  modelSource?: "role_default" | "company_override" | "task_override";
};

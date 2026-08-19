import { invokeTauri } from "@/shared/api/tauri";
import type {
  WorkforceCommunityDraftInput,
  WorkforceContextFact,
  WorkforceContextReviewDecision,
  WorkforceModelRoute,
  WorkforceRoutePreview,
  WorkforceSummary,
} from "@/shared/api/workforceTypes";

export function getWorkforceSummary(): Promise<WorkforceSummary> {
  return invokeTauri("get_workforce_summary");
}

export function upsertWorkforceDraftCommunity(
  input: WorkforceCommunityDraftInput,
  expectedRevision: number,
): Promise<WorkforceSummary> {
  return invokeTauri("upsert_workforce_draft_community", {
    input,
    expectedRevision,
  });
}

export function setWorkforceCommunityMembership(input: {
  companyId: string;
  identityId: string;
  enabled: boolean;
  startOnAppLaunch: boolean;
  expectedRevision: number;
}): Promise<WorkforceSummary> {
  return invokeTauri("set_workforce_community_membership", input);
}

export function setWorkforceCompanyModelOverride(input: {
  companyId: string;
  roleId: string;
  route: WorkforceModelRoute;
  expectedRevision: number;
}): Promise<WorkforceSummary> {
  return invokeTauri("set_workforce_company_model_override", input);
}

export function proposeWorkforceContextFact(input: {
  companyId: string;
  fact: WorkforceContextFact;
  expectedRevision: number;
}): Promise<number> {
  return invokeTauri("propose_workforce_context_fact", input);
}

export function reviewWorkforceContextFact(input: {
  companyId: string;
  factId: string;
  decision: WorkforceContextReviewDecision;
  expectedRevision: number;
}): Promise<number> {
  return invokeTauri("review_workforce_context_fact", input);
}

export function previewWorkforceTaskRoute(input: {
  pubkey: string;
  relayUrl: string;
  taskModelOverride: WorkforceModelRoute;
}): Promise<WorkforceRoutePreview> {
  return invokeTauri("preview_workforce_task_route", input);
}

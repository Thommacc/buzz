import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import { managedAgentRuntimesQueryKey } from "@/features/agents/managedAgentRuntimeHooks";
import { managedAgentsQueryKey } from "@/features/agents/hooks";
import {
  getWorkforceSummary,
  proposeWorkforceContextFact,
  reviewWorkforceContextFact,
  setWorkforceCommunityMembership,
  setWorkforceCompanyModelOverride,
  upsertWorkforceDraftCommunity,
} from "@/shared/api/tauriWorkforce";
import type { WorkforceSummary } from "@/shared/api/workforceTypes";

export const workforceQueryKey = ["workforce"] as const;

export function useWorkforceQuery() {
  return useQuery({
    queryKey: workforceQueryKey,
    queryFn: getWorkforceSummary,
    staleTime: 30_000,
  });
}

function useWorkforceMutation<TVariables>(
  mutationFn: (variables: TVariables) => Promise<WorkforceSummary>,
) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn,
    onSuccess: (summary) => {
      queryClient.setQueryData(workforceQueryKey, summary);
    },
    onSettled: async () => {
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: workforceQueryKey }),
        queryClient.invalidateQueries({ queryKey: managedAgentsQueryKey }),
        queryClient.invalidateQueries({
          queryKey: managedAgentRuntimesQueryKey,
        }),
      ]);
    },
  });
}

export function useUpsertWorkforceDraftCommunityMutation() {
  return useWorkforceMutation(
    (variables: {
      input: Parameters<typeof upsertWorkforceDraftCommunity>[0];
      expectedRevision: number;
    }) =>
      upsertWorkforceDraftCommunity(
        variables.input,
        variables.expectedRevision,
      ),
  );
}

export function useSetWorkforceCommunityMembershipMutation() {
  return useWorkforceMutation(setWorkforceCommunityMembership);
}

export function useSetWorkforceCompanyModelOverrideMutation() {
  return useWorkforceMutation(setWorkforceCompanyModelOverride);
}

export function useProposeWorkforceContextFactMutation() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: proposeWorkforceContextFact,
    onSettled: async () => {
      await queryClient.invalidateQueries({ queryKey: workforceQueryKey });
    },
  });
}

export function useReviewWorkforceContextFactMutation() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: reviewWorkforceContextFact,
    onSettled: async () => {
      await queryClient.invalidateQueries({ queryKey: workforceQueryKey });
    },
  });
}

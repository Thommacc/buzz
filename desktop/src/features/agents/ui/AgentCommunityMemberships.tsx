import {
  useSetWorkforceCommunityMembershipMutation,
  useWorkforceQuery,
} from "@/features/agents/workforceHooks";
import { useManagedAgentRuntimesQuery } from "@/features/agents/managedAgentRuntimeHooks";
import { Badge } from "@/shared/ui/badge";
import { Button } from "@/shared/ui/button";
import {
  partitionWorkforceIdentities,
  resolveVisibleWorkforceModel,
} from "./workforcePresentation";

export function AgentCommunityMemberships() {
  const workforce = useWorkforceQuery();
  const membershipMutation = useSetWorkforceCommunityMembershipMutation();
  const runtimesQuery = useManagedAgentRuntimesQuery();
  const summary = workforce.data;

  if (workforce.isPending) {
    return (
      <p className="text-sm text-muted-foreground">Loading AI workforce…</p>
    );
  }
  if (workforce.error instanceof Error) {
    return (
      <p className="text-sm text-destructive">{workforce.error.message}</p>
    );
  }
  if (!summary || summary.identities.length === 0) return null;

  const { employees, hermes } = partitionWorkforceIdentities(
    summary.identities,
  );

  async function toggleMembership(
    companyId: string,
    identityId: string,
    enabled: boolean,
  ) {
    if (!summary) return;
    await membershipMutation.mutateAsync({
      companyId,
      identityId,
      enabled,
      startOnAppLaunch: false,
      expectedRevision: summary.revision,
    });
  }

  return (
    <div className="mx-auto w-full max-w-[996px] space-y-4 rounded-xl border border-border/60 p-4">
      <div>
        <h2 className="text-sm font-semibold">AI workforce by community</h2>
        <p className="text-xs text-muted-foreground">
          Each person appears once. Community context and model routing are
          applied only when that membership runs.
        </p>
      </div>
      <IdentitySection
        title="AI employees"
        identities={employees}
        summary={summary}
        runtimes={runtimesQuery.data ?? []}
        isPending={membershipMutation.isPending}
        onToggle={toggleMembership}
      />
      <IdentitySection
        title="Hermes agents"
        description="References to your existing Hermes identities; Buzz does not copy or replace their prompt, model, tools, or runtime."
        identities={hermes}
        summary={summary}
        runtimes={runtimesQuery.data ?? []}
        isPending={membershipMutation.isPending}
        onToggle={toggleMembership}
      />
      {membershipMutation.error instanceof Error ? (
        <p className="text-sm text-destructive">
          {membershipMutation.error.message}
        </p>
      ) : null}
    </div>
  );
}

function IdentitySection({
  title,
  description,
  identities,
  summary,
  runtimes,
  isPending,
  onToggle,
}: {
  title: string;
  description?: string;
  identities: NonNullable<
    ReturnType<typeof useWorkforceQuery>["data"]
  >["identities"];
  summary: NonNullable<ReturnType<typeof useWorkforceQuery>["data"]>;
  runtimes: NonNullable<
    ReturnType<typeof useManagedAgentRuntimesQuery>["data"]
  >;
  isPending: boolean;
  onToggle: (
    companyId: string,
    identityId: string,
    enabled: boolean,
  ) => Promise<void>;
}) {
  if (identities.length === 0) return null;
  return (
    <section className="space-y-2">
      <div>
        <h3 className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
          {title}
        </h3>
        {description ? (
          <p className="text-xs text-muted-foreground">{description}</p>
        ) : null}
      </div>
      {identities.map((identity) => (
        <div
          className="rounded-lg border border-border/60 p-3"
          data-testid={`workforce-identity-${identity.identityId}`}
          key={identity.identityId}
        >
          <div className="flex flex-wrap items-start justify-between gap-2">
            <div>
              <p className="text-sm font-medium">
                {identity.role?.roleId ?? identity.identityId}
              </p>
              <p className="text-xs text-muted-foreground">
                {identity.role
                  ? `${identity.role.department} · ${identity.role.useWhen}`
                  : `Hermes profile: ${identity.hermesProfileRef ?? "linked identity"}`}
              </p>
            </div>
            {identity.role ? (
              <Badge variant="secondary">
                Default: {identity.role.defaultModel.model}
              </Badge>
            ) : (
              <Badge variant="outline">Hermes-owned runtime</Badge>
            )}
          </div>
          <div className="mt-3 flex flex-wrap gap-2">
            {summary.communities.map((community) => {
              const membership = community.memberships.find(
                (item) => item.identityId === identity.identityId,
              );
              const enabled = membership?.enabled ?? false;
              const resolvedModel = resolveVisibleWorkforceModel(
                identity,
                community,
              );
              const runtime = community.relayUrl
                ? runtimes.find(
                    (item) =>
                      item.pubkey.toLowerCase() ===
                        identity.pubkey.toLowerCase() &&
                      item.relayUrl.toLowerCase() ===
                        community.relayUrl?.toLowerCase(),
                  )
                : undefined;
              return (
                <div
                  className="flex items-center gap-2 rounded-md bg-muted/40 px-2 py-1.5"
                  key={community.companyId}
                >
                  <div>
                    <div className="flex items-center gap-1.5">
                      <span className="text-xs font-medium">
                        {community.displayName}
                      </span>
                      <Badge variant={enabled ? "default" : "outline"}>
                        {enabled ? "Member" : "Not added"}
                      </Badge>
                      {community.rolloutState === "draft" ||
                      community.rolloutState === "canary" ? (
                        <Badge variant="warning">
                          {community.rolloutState} · stopped
                        </Badge>
                      ) : null}
                      {enabled ? (
                        <Badge variant="secondary">
                          Runtime: {runtime?.lifecycle ?? "stopped"}
                        </Badge>
                      ) : null}
                    </div>
                    {resolvedModel ? (
                      <p className="text-2xs text-muted-foreground">
                        {resolvedModel.route.model} · {resolvedModel.reason}
                      </p>
                    ) : null}
                  </div>
                  <Button
                    disabled={isPending}
                    onClick={() =>
                      void onToggle(
                        community.companyId,
                        identity.identityId,
                        !enabled,
                      )
                    }
                    size="sm"
                    type="button"
                    variant="outline"
                  >
                    {enabled ? "Remove" : "Add"}
                  </Button>
                </div>
              );
            })}
          </div>
        </div>
      ))}
    </section>
  );
}

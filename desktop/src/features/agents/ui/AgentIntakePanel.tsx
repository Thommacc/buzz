import * as React from "react";

import {
  recommendRoles,
  roleCatalog,
  type RoleCatalogEntry,
} from "@/features/agents/intake/roleIntake";
import { useWorkforceQuery } from "@/features/agents/workforceHooks";
import { Badge } from "@/shared/ui/badge";
import { Button } from "@/shared/ui/button";
import { Input } from "@/shared/ui/input";
import { Textarea } from "@/shared/ui/textarea";

export function AgentIntakePanel({
  activeRelayUrl,
}: {
  activeRelayUrl?: string;
}) {
  const workforceQuery = useWorkforceQuery();
  const [task, setTask] = React.useState("");
  const [search, setSearch] = React.useState("");
  const [submittedTask, setSubmittedTask] = React.useState("");
  const workforceCommunity = workforceQuery.data?.communities.find(
    (community) =>
      community.relayUrl?.toLowerCase() === activeRelayUrl?.toLowerCase(),
  );
  const recommendation =
    submittedTask && workforceQuery.data && workforceCommunity
      ? recommendRoles(
          submittedTask,
          workforceQuery.data,
          workforceCommunity.companyId,
        )
      : null;
  const normalizedSearch = search.trim().toLocaleLowerCase();
  const visibleRoles = roleCatalog.filter((role) => {
    if (!normalizedSearch) return true;
    return [
      role.displayName,
      role.roleId,
      role.department,
      role.useWhen,
      ...role.keywords,
    ].some((value) => value.toLocaleLowerCase().includes(normalizedSearch));
  });

  return (
    <section className="mx-auto w-full max-w-[996px] space-y-4 rounded-xl border border-border/60 p-4">
      <div>
        <h2 className="text-sm font-semibold">AI Team - Intake &amp; Regie</h2>
        <p className="text-xs text-muted-foreground">
          Describe the outcome. Intake recommends only employees enabled in the
          active registered community.
        </p>
      </div>
      <div className="flex items-end gap-2">
        <Textarea
          aria-label="Describe the work"
          onChange={(event) => setTask(event.target.value)}
          placeholder="For example: check this invoice and prepare a management report"
          value={task}
        />
        <Button
          disabled={!task.trim() || !workforceCommunity}
          onClick={() => setSubmittedTask(task.trim())}
          type="button"
        >
          Advise
        </Button>
      </div>
      {!workforceCommunity ? (
        <p className="text-xs text-muted-foreground">
          The active Buzz community is not yet linked to a workforce company.
          You can still browse all 35 roles below.
        </p>
      ) : submittedTask && !recommendation ? (
        <p className="text-xs text-muted-foreground">
          No enabled role matches this task in {workforceCommunity.displayName}.
        </p>
      ) : recommendation ? (
        <div className="space-y-2 rounded-lg bg-muted/40 p-3">
          <RecommendationRow
            label="Primary"
            reason={recommendation.reasons[recommendation.primary.identityId]}
            role={recommendation.primary}
          />
          {recommendation.supporting.map((role) => (
            <RecommendationRow
              key={role.identityId}
              label="Support"
              reason={recommendation.reasons[role.identityId]}
              role={role}
            />
          ))}
          <p className="text-xs text-muted-foreground">
            Confirm the selection before external, financial, legal, personnel,
            credential, destructive, or production-impacting action.
          </p>
        </div>
      ) : null}

      <div className="space-y-2">
        <Input
          aria-label="Search AI employee roles"
          onChange={(event) => setSearch(event.target.value)}
          placeholder="Search 35 roles by name, department, or task"
          value={search}
        />
        <div className="grid max-h-72 grid-cols-1 gap-2 overflow-y-auto sm:grid-cols-2">
          {visibleRoles.map((role) => (
            <RoleSummary key={role.identityId} role={role} />
          ))}
        </div>
      </div>
    </section>
  );
}

function RecommendationRow({
  label,
  role,
  reason,
}: {
  label: string;
  role: RoleCatalogEntry;
  reason: string;
}) {
  return (
    <div className="rounded-md border border-border/60 bg-background p-2">
      <div className="flex flex-wrap items-center gap-2">
        <Badge variant={label === "Primary" ? "default" : "secondary"}>
          {label}
        </Badge>
        <span className="text-sm font-medium">{role.displayName}</span>
        <span className="text-xs text-muted-foreground">
          {role.defaultModel.model}
        </span>
      </div>
      <p className="mt-1 text-xs">{reason}</p>
      <p className="text-xs text-muted-foreground">
        Input: {role.requiredInputs.join(", ")} · Output:{" "}
        {role.expectedOutputs.join(", ")}
      </p>
      <p className="text-xs text-muted-foreground">
        Approval: {role.approvalBoundary}
      </p>
    </div>
  );
}

function RoleSummary({ role }: { role: RoleCatalogEntry }) {
  return (
    <article className="rounded-md border border-border/60 p-2">
      <div className="flex flex-wrap items-center gap-1.5">
        <p className="text-sm font-medium">{role.displayName}</p>
        <Badge variant="outline">{role.department}</Badge>
      </div>
      <p className="text-xs text-muted-foreground">{role.useWhen}</p>
      <p className="mt-1 text-xs text-muted-foreground">
        {role.defaultModel.model} · {role.defaultModel.health}
      </p>
    </article>
  );
}

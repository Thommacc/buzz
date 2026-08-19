use crate::app_state::AppState;
use crate::managed_agents::workforce::*;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommunityDraftInput {
    pub company_id: String,
    pub display_name: String,
    pub default_language: String,
    #[serde(default)]
    pub owners: Vec<String>,
    #[serde(default)]
    pub approvers: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkforceSummary {
    pub schema_version: u32,
    pub revision: u64,
    pub identities: Vec<WorkforceIdentitySummary>,
    pub communities: Vec<CommunityRecord>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkforceIdentitySummary {
    pub identity_id: String,
    pub pubkey: String,
    pub kind: WorkforceIdentityKind,
    pub role: Option<WorkforceRoleSummary>,
    pub hermes_profile_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkforceRoleSummary {
    pub role_id: String,
    pub department: String,
    pub use_when: String,
    pub required_inputs: Vec<String>,
    pub expected_outputs: Vec<String>,
    pub default_model: ModelRoute,
    pub approval_boundary: String,
    pub catalog_version: u32,
    pub last_verified_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case", tag = "decision")]
pub enum ContextReviewDecision {
    Approve,
    Reject { reason: String },
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkforceRoutePreview {
    pub company_id: String,
    pub identity_id: String,
    pub role_id: Option<String>,
    pub workforce_revision: u64,
    pub context_version: u64,
    pub model: Option<ModelRoute>,
    pub model_source: Option<WorkforceModelSource>,
}

impl From<ResolvedWorkforceExecution> for WorkforceRoutePreview {
    fn from(execution: ResolvedWorkforceExecution) -> Self {
        Self {
            company_id: execution.company_id,
            identity_id: execution.identity_id,
            role_id: execution.role_id,
            workforce_revision: execution.workforce_revision,
            context_version: execution.context_version,
            model: execution.model,
            model_source: execution.model_source,
        }
    }
}

fn require_revision(actual: u64, expected: u64) -> Result<(), String> {
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "stale workforce edit: expected revision {expected}, current revision is {actual}"
        ))
    }
}

fn upsert_draft_community(
    store: &mut WorkforceStore,
    input: CommunityDraftInput,
    expected_revision: u64,
) -> Result<(), String> {
    require_revision(store.revision, expected_revision)?;
    validate_identifier("companyId", &input.company_id).map_err(|error| error.to_string())?;
    if input.display_name.trim().is_empty() || input.default_language.trim().is_empty() {
        return Err("community display name and default language are required".into());
    }
    if let Some(existing) = store
        .communities
        .iter_mut()
        .find(|community| community.company_id == input.company_id)
    {
        if existing.rollout_state != RolloutState::Draft {
            return Err("only draft communities can be changed through draft upsert".into());
        }
        existing.display_name = input.display_name;
        existing.default_language = input.default_language;
        existing.owners = input.owners;
        existing.approvers = input.approvers;
    } else {
        store.communities.push(CommunityRecord {
            company_id: input.company_id.clone(),
            display_name: input.display_name,
            relay_url: None,
            context_ref: format!("contexts/{}.json", input.company_id),
            default_language: input.default_language,
            owners: input.owners,
            approvers: input.approvers,
            rollout_state: RolloutState::Draft,
            model_policy: ModelPolicy::default(),
            memberships: Vec::new(),
        });
    }
    store.revision = store.revision.saturating_add(1);
    store.validate().map_err(|error| error.to_string())?;
    Ok(())
}

fn set_membership(
    store: &mut WorkforceStore,
    company_id: &str,
    identity_id: &str,
    enabled: bool,
    start_on_app_launch: bool,
    expected_revision: u64,
) -> Result<(), String> {
    require_revision(store.revision, expected_revision)?;
    if !store
        .identities
        .iter()
        .any(|identity| identity.identity_id == identity_id)
    {
        return Err(format!("workforce identity {identity_id:?} not found"));
    }
    let community = store
        .communities
        .iter_mut()
        .find(|community| community.company_id == company_id)
        .ok_or_else(|| format!("community {company_id:?} not found"))?;
    if let Some(membership) = community
        .memberships
        .iter_mut()
        .find(|membership| membership.identity_id == identity_id)
    {
        membership.enabled = enabled;
        membership.start_on_app_launch = enabled && start_on_app_launch;
    } else {
        community.memberships.push(CommunityMembership {
            identity_id: identity_id.into(),
            enabled,
            start_on_app_launch: enabled && start_on_app_launch,
        });
    }
    store.revision = store.revision.saturating_add(1);
    store.validate().map_err(|error| error.to_string())?;
    Ok(())
}

fn set_company_model_override(
    store: &mut WorkforceStore,
    company_id: &str,
    role_id: &str,
    route: ModelRoute,
    expected_revision: u64,
) -> Result<(), String> {
    require_revision(store.revision, expected_revision)?;
    validate_model_route(&route)?;
    if !store.identities.iter().any(|identity| {
        identity
            .role
            .as_ref()
            .is_some_and(|role| role.role_id == role_id)
    }) {
        return Err(format!("role {role_id:?} not found"));
    }
    let community = store
        .communities
        .iter_mut()
        .find(|community| community.company_id == company_id)
        .ok_or_else(|| format!("community {company_id:?} not found"))?;
    community
        .model_policy
        .role_overrides
        .insert(role_id.into(), route);
    store.revision = store.revision.saturating_add(1);
    store.validate().map_err(|error| error.to_string())?;
    Ok(())
}

fn propose_context_fact(
    context: &mut CompanyContext,
    fact: ContextFact,
    expected_revision: u64,
) -> Result<(), String> {
    require_revision(context.revision, expected_revision)?;
    context.propose_fact(fact)
}

fn review_context_fact(
    context: &mut CompanyContext,
    fact_id: &str,
    decision: ContextReviewDecision,
    reviewer: &str,
    reviewed_at: &str,
    expected_revision: u64,
) -> Result<(), String> {
    require_revision(context.revision, expected_revision)?;
    match decision {
        ContextReviewDecision::Approve => context.approve_fact(fact_id, reviewer, reviewed_at),
        ContextReviewDecision::Reject { reason } => {
            context.reject_fact(fact_id, reviewer, reviewed_at, &reason)
        }
    }
}

fn workforce_summary(store: &WorkforceStore) -> WorkforceSummary {
    WorkforceSummary {
        schema_version: store.schema_version,
        revision: store.revision,
        identities: store
            .identities
            .iter()
            .map(|identity| WorkforceIdentitySummary {
                identity_id: identity.identity_id.clone(),
                pubkey: identity.pubkey.clone(),
                kind: identity.kind.clone(),
                role: identity.role.as_ref().map(|role| WorkforceRoleSummary {
                    role_id: role.role_id.clone(),
                    department: role.department.clone(),
                    use_when: role.use_when.clone(),
                    required_inputs: role.required_inputs.clone(),
                    expected_outputs: role.expected_outputs.clone(),
                    default_model: role.default_model.clone(),
                    approval_boundary: role.approval_boundary.clone(),
                    catalog_version: role.catalog_version,
                    last_verified_at: role.last_verified_at.clone(),
                }),
                hermes_profile_ref: identity.hermes_profile_ref.clone(),
            })
            .collect(),
        communities: store.communities.clone(),
    }
}

#[tauri::command]
pub fn get_workforce_summary(app: AppHandle) -> Result<WorkforceSummary, String> {
    Ok(workforce_summary(&load_workforce(&app)?))
}

#[tauri::command]
pub fn upsert_workforce_draft_community(
    input: CommunityDraftInput,
    expected_revision: u64,
    app: AppHandle,
) -> Result<WorkforceSummary, String> {
    let state = app.state::<AppState>();
    let _guard = state
        .managed_agents_store_lock
        .lock()
        .map_err(|error| error.to_string())?;
    let mut store = load_workforce(&app)?;
    upsert_draft_community(&mut store, input.clone(), expected_revision)?;
    ensure_company_context(&app, &input.company_id)?;
    save_workforce(&app, &store)?;
    Ok(workforce_summary(&store))
}

#[tauri::command]
pub fn set_workforce_community_membership(
    company_id: String,
    identity_id: String,
    enabled: bool,
    start_on_app_launch: bool,
    expected_revision: u64,
    app: AppHandle,
) -> Result<WorkforceSummary, String> {
    let state = app.state::<AppState>();
    let _guard = state
        .managed_agents_store_lock
        .lock()
        .map_err(|error| error.to_string())?;
    let mut store = load_workforce(&app)?;
    set_membership(
        &mut store,
        &company_id,
        &identity_id,
        enabled,
        start_on_app_launch,
        expected_revision,
    )?;
    save_workforce(&app, &store)?;
    Ok(workforce_summary(&store))
}

#[tauri::command]
pub fn set_workforce_company_model_override(
    company_id: String,
    role_id: String,
    route: ModelRoute,
    expected_revision: u64,
    app: AppHandle,
) -> Result<WorkforceSummary, String> {
    let state = app.state::<AppState>();
    let _guard = state
        .managed_agents_store_lock
        .lock()
        .map_err(|error| error.to_string())?;
    let mut store = load_workforce(&app)?;
    set_company_model_override(&mut store, &company_id, &role_id, route, expected_revision)?;
    save_workforce(&app, &store)?;
    Ok(workforce_summary(&store))
}

#[tauri::command]
pub fn propose_workforce_context_fact(
    company_id: String,
    fact: ContextFact,
    expected_revision: u64,
    app: AppHandle,
) -> Result<u64, String> {
    let state = app.state::<AppState>();
    let _guard = state
        .managed_agents_store_lock
        .lock()
        .map_err(|error| error.to_string())?;
    let mut context = load_company_context(&app, &company_id)?;
    propose_context_fact(&mut context, fact, expected_revision)?;
    save_company_context(&app, &context)?;
    Ok(context.revision)
}

#[tauri::command]
pub fn review_workforce_context_fact(
    company_id: String,
    fact_id: String,
    decision: ContextReviewDecision,
    expected_revision: u64,
    app: AppHandle,
) -> Result<u64, String> {
    let state = app.state::<AppState>();
    let reviewer = state
        .keys
        .lock()
        .map_err(|error| error.to_string())?
        .public_key()
        .to_hex();
    let _guard = state
        .managed_agents_store_lock
        .lock()
        .map_err(|error| error.to_string())?;
    let mut context = load_company_context(&app, &company_id)?;
    review_context_fact(
        &mut context,
        &fact_id,
        decision,
        &reviewer,
        &crate::util::now_iso(),
        expected_revision,
    )?;
    save_company_context(&app, &context)?;
    Ok(context.revision)
}

#[tauri::command]
pub fn preview_workforce_task_route(
    pubkey: String,
    relay_url: String,
    task_model_override: ModelRoute,
    app: AppHandle,
) -> Result<WorkforceRoutePreview, String> {
    validate_model_route(&task_model_override)?;
    let execution = resolve_workforce_execution_for_app(
        &app,
        &pubkey,
        &relay_url,
        Some(&task_model_override),
        None,
    )?
    .ok_or_else(|| "identity is not enrolled in workforce mode".to_string())?;
    Ok(execution.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn route(model: &str) -> ModelRoute {
        ModelRoute {
            provider: "openai".into(),
            model: model.into(),
            purpose_label: "General".into(),
            approved: true,
            health: ModelRouteHealth::Healthy,
            last_health_check_at: "2026-08-19T00:00:00Z".into(),
        }
    }

    fn store() -> WorkforceStore {
        WorkforceStore {
            schema_version: WORKFORCE_SCHEMA_VERSION,
            revision: 4,
            identities: vec![WorkforceIdentity {
                identity_id: "faye".into(),
                pubkey: "aa".repeat(32),
                kind: WorkforceIdentityKind::Employee,
                role: Some(EmployeeRole {
                    role_id: "finance-controller".into(),
                    department: "Finance".into(),
                    use_when: "Finance control".into(),
                    required_inputs: vec![],
                    expected_outputs: vec![],
                    general_prompt: "Control finance".into(),
                    default_model: route("role-default"),
                    approval_boundary: "Ask first".into(),
                    catalog_version: 1,
                    last_verified_at: "2026-08-19T00:00:00Z".into(),
                }),
                hermes_profile_ref: None,
            }],
            communities: vec![],
        }
    }

    #[test]
    fn upsert_draft_is_data_only_and_revision_guarded() {
        let mut store = store();
        upsert_draft_community(
            &mut store,
            CommunityDraftInput {
                company_id: "dsry".into(),
                display_name: "DSRY".into(),
                default_language: "en".into(),
                owners: vec!["thom".into()],
                approvers: vec!["thom".into()],
            },
            4,
        )
        .expect("upsert");
        assert_eq!(store.revision, 5);
        let community = &store.communities[0];
        assert_eq!(community.rollout_state, RolloutState::Draft);
        assert!(community.relay_url.is_none());
        assert!(upsert_draft_community(
            &mut store,
            CommunityDraftInput {
                company_id: "damen".into(),
                display_name: "Damen".into(),
                default_language: "nl".into(),
                owners: vec![],
                approvers: vec![],
            },
            4,
        )
        .is_err());
    }

    #[test]
    fn membership_mutation_enables_without_autostart() {
        let mut store = store();
        upsert_draft_community(
            &mut store,
            CommunityDraftInput {
                company_id: "dsry".into(),
                display_name: "DSRY".into(),
                default_language: "en".into(),
                owners: vec![],
                approvers: vec![],
            },
            4,
        )
        .expect("community");
        set_membership(&mut store, "dsry", "faye", true, false, 5).expect("membership");
        assert!(store.communities[0].memberships[0].enabled);
        assert!(!store.communities[0].memberships[0].start_on_app_launch);
        set_membership(&mut store, "dsry", "faye", false, true, 6).expect("disable");
        assert!(!store.communities[0].memberships[0].enabled);
        assert!(!store.communities[0].memberships[0].start_on_app_launch);
    }

    #[test]
    fn model_override_rejects_unknown_role_and_unhealthy_route() {
        let mut store = store();
        upsert_draft_community(
            &mut store,
            CommunityDraftInput {
                company_id: "dsry".into(),
                display_name: "DSRY".into(),
                default_language: "en".into(),
                owners: vec![],
                approvers: vec![],
            },
            4,
        )
        .expect("community");
        assert!(
            set_company_model_override(&mut store, "dsry", "missing-role", route("model"), 5,)
                .is_err()
        );
        let mut unhealthy = route("model");
        unhealthy.health = ModelRouteHealth::Unhealthy;
        assert!(
            set_company_model_override(&mut store, "dsry", "finance-controller", unhealthy, 5,)
                .is_err()
        );
        set_company_model_override(
            &mut store,
            "dsry",
            "finance-controller",
            route("company-model"),
            5,
        )
        .expect("healthy company route");
        assert_eq!(
            store.communities[0].model_policy.role_overrides["finance-controller"].model,
            "company-model"
        );
    }

    #[test]
    fn context_mutations_use_context_revision() {
        let mut context = CompanyContext {
            company_id: "dsry".into(),
            version: 0,
            revision: 2,
            approved_at: None,
            facts: vec![],
            documents: vec![],
            templates: Default::default(),
        };
        propose_context_fact(
            &mut context,
            ContextFact {
                fact_id: "yard-location".into(),
                statement: "Located in Djibouti".into(),
                source: "source".into(),
                proposed_at: "2026-08-19T00:00:00Z".into(),
                confidence_basis_points: 9000,
                status: ContextFactStatus::Proposed,
            },
            2,
        )
        .expect("propose");
        assert_eq!(context.revision, 3);
        review_context_fact(
            &mut context,
            "yard-location",
            ContextReviewDecision::Approve,
            "thom",
            "2026-08-19T01:00:00Z",
            3,
        )
        .expect("approve");
        assert_eq!(context.version, 1);
        assert_eq!(context.revision, 4);
    }

    #[test]
    fn summary_excludes_role_prompts_and_context_bodies() {
        let store = store();
        let summary = workforce_summary(&store);
        let json = serde_json::to_string(&summary).expect("summary json");
        assert!(json.contains("finance-controller"));
        assert!(!json.contains("Control finance"));
    }

    #[test]
    fn task_override_preview_is_redacted() {
        let mut invalid = route("task-model");
        invalid.approved = false;
        assert!(validate_model_route(&invalid).is_err());
        let execution = ResolvedWorkforceExecution {
            company_id: "dsry".into(),
            identity_id: "faye".into(),
            role_id: Some("finance-controller".into()),
            hermes_profile_ref: None,
            workforce_revision: 4,
            context_version: 2,
            role_version: Some(1),
            system_prompt: Some("SECRET CONTEXT BODY".into()),
            model: Some(route("task-model")),
            model_source: Some(WorkforceModelSource::TaskOverride),
            role_hash: Some("role-hash".into()),
            context_hash: "context-hash".into(),
            prompt_hash: Some("prompt-hash".into()),
        };
        let preview = WorkforceRoutePreview::from(execution);
        let json = serde_json::to_string(&preview).expect("preview json");
        assert!(json.contains("task-model"));
        assert!(!json.contains("SECRET CONTEXT BODY"));
    }
}

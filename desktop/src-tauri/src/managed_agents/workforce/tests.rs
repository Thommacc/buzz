use std::collections::BTreeMap;

use super::*;

fn model(model: &str) -> ModelRoute {
    ModelRoute {
        provider: "openai".into(),
        model: model.into(),
        purpose_label: "General work".into(),
        approved: true,
        health: ModelRouteHealth::Healthy,
        last_health_check_at: "2026-08-19T00:00:00Z".into(),
    }
}

fn employee(identity_id: &str, pubkey: &str, role_id: &str) -> WorkforceIdentity {
    WorkforceIdentity {
        identity_id: identity_id.into(),
        pubkey: pubkey.into(),
        kind: WorkforceIdentityKind::Employee,
        role: Some(EmployeeRole {
            role_id: role_id.into(),
            department: "Operations".into(),
            use_when: "Operational coordination is needed".into(),
            required_inputs: vec!["desired outcome".into()],
            expected_outputs: vec!["action plan".into()],
            general_prompt: "Coordinate operational work.".into(),
            default_model: model("gpt-role-default"),
            approval_boundary: "Ask before external action.".into(),
            catalog_version: 1,
            last_verified_at: "2026-08-19T00:00:00Z".into(),
        }),
        hermes_profile_ref: None,
    }
}

fn community(company_id: &str, relay_url: Option<&str>) -> CommunityRecord {
    CommunityRecord {
        company_id: company_id.into(),
        display_name: company_id.into(),
        relay_url: relay_url.map(str::to_owned),
        context_ref: format!("contexts/{company_id}.json"),
        default_language: "nl".into(),
        owners: vec!["thom".into()],
        approvers: vec!["thom".into()],
        rollout_state: if relay_url.is_some() {
            RolloutState::Active
        } else {
            RolloutState::Draft
        },
        model_policy: ModelPolicy::default(),
        memberships: Vec::new(),
    }
}

fn valid_store() -> WorkforceStore {
    let identity = employee("ops", &"aa".repeat(32), "operations-coordinator");
    let mut community = community("acme", Some("WSS://Relay.Example:443/"));
    community.memberships.push(CommunityMembership {
        identity_id: identity.identity_id.clone(),
        enabled: true,
        start_on_app_launch: false,
    });
    WorkforceStore {
        schema_version: WORKFORCE_SCHEMA_VERSION,
        revision: 1,
        identities: vec![identity],
        communities: vec![community],
    }
}

#[test]
fn rejects_unknown_schema_version() {
    let mut store = valid_store();
    store.schema_version += 1;
    assert!(matches!(
        store.validate(),
        Err(WorkforceValidationError::UnsupportedSchemaVersion { .. })
    ));
}

#[test]
fn rejects_duplicate_company_ids() {
    let mut store = valid_store();
    store.communities.push(community("acme", None));
    assert!(matches!(
        store.validate(),
        Err(WorkforceValidationError::DuplicateCompanyId(id)) if id == "acme"
    ));
}

#[test]
fn rejects_duplicate_canonical_relays() {
    let mut store = valid_store();
    store
        .communities
        .push(community("other", Some("wss://relay.example")));
    assert!(matches!(
        store.validate(),
        Err(WorkforceValidationError::DuplicateRelayUrl { .. })
    ));
}

#[test]
fn rejects_unknown_membership_identity() {
    let mut store = valid_store();
    store.communities[0].memberships[0].identity_id = "missing".into();
    assert!(matches!(
        store.validate(),
        Err(WorkforceValidationError::UnknownIdentityReference { .. })
    ));
}

#[test]
fn rejects_model_override_for_unknown_role() {
    let mut store = valid_store();
    store.communities[0]
        .model_policy
        .role_overrides
        .insert("unknown-role".into(), model("gpt-company"));
    assert!(matches!(
        store.validate(),
        Err(WorkforceValidationError::UnknownRoleReference { .. })
    ));
}

#[test]
fn invalid_rollout_state_fails_deserialization() {
    let json = serde_json::to_string(&valid_store()).expect("serialize fixture");
    let invalid = json.replace("\"active\"", "\"launching\"");
    assert!(serde_json::from_str::<WorkforceStore>(&invalid).is_err());
}

#[test]
fn approved_context_excludes_untrusted_facts() {
    let context = CompanyContext {
        company_id: "acme".into(),
        version: 4,
        revision: 4,
        approved_at: Some("2026-08-19T00:00:00Z".into()),
        facts: vec![
            ContextFact {
                fact_id: "approved".into(),
                statement: "Approved fact".into(),
                source: "source-a".into(),
                proposed_at: "2026-08-18T00:00:00Z".into(),
                confidence_basis_points: 9000,
                status: ContextFactStatus::Approved {
                    reviewer: "thom".into(),
                    reviewed_at: "2026-08-19T00:00:00Z".into(),
                },
            },
            ContextFact {
                fact_id: "proposed".into(),
                statement: "Untrusted proposal".into(),
                source: "source-b".into(),
                proposed_at: "2026-08-19T00:00:00Z".into(),
                confidence_basis_points: 5000,
                status: ContextFactStatus::Proposed,
            },
        ],
        documents: Vec::new(),
        templates: BTreeMap::new(),
    };
    let facts: Vec<_> = context
        .approved_facts()
        .map(|fact| fact.fact_id.as_str())
        .collect();
    assert_eq!(facts, vec!["approved"]);
}

#[test]
fn hermes_identity_is_reference_only() {
    let mut store = valid_store();
    store.identities.push(WorkforceIdentity {
        identity_id: "nova".into(),
        pubkey: "bb".repeat(32),
        kind: WorkforceIdentityKind::Hermes,
        role: None,
        hermes_profile_ref: Some("hermes:nova".into()),
    });
    assert!(store.validate().is_ok());

    store.identities[1].role = store.identities[0].role.clone();
    assert!(matches!(
        store.validate(),
        Err(WorkforceValidationError::InvalidHermesIdentity(_))
    ));
}

#[test]
fn active_community_requires_a_relay_but_draft_does_not() {
    let mut store = valid_store();
    store.communities[0].relay_url = None;
    assert!(matches!(
        store.validate(),
        Err(WorkforceValidationError::MissingActiveRelay(_))
    ));
    store.communities[0].rollout_state = RolloutState::Draft;
    assert!(store.validate().is_ok());
}

#[test]
fn canonical_relay_lookup_returns_the_authenticated_company() {
    let store = valid_store();
    let validated = store.validate().expect("valid store");
    let company = validated
        .community_for_relay("wss://relay.example/")
        .expect("registered relay");
    assert_eq!(company.company_id, "acme");
    assert!(validated
        .community_for_relay("wss://unknown.example")
        .is_none());
}

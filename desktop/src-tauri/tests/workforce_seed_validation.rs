use std::collections::{BTreeMap, HashSet};

use buzz_lib::workforce_schema::{CompanyContext, WorkforceStore};
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Catalog {
    catalog_version: u32,
    roles: Vec<Role>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Role {
    identity_id: String,
    persona_id: String,
    role_id: String,
    department: String,
    use_when: String,
    required_inputs: Vec<String>,
    expected_outputs: Vec<String>,
    general_prompt: String,
    default_model: Value,
    approval_boundary: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Registry {
    companies: Vec<Company>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Company {
    company_id: String,
    display_name: String,
    relay_url: Option<String>,
    context_ref: String,
    default_language: String,
    rollout_state: String,
}

#[derive(Deserialize)]
struct HermesSeed {
    identities: Vec<HermesIdentity>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct HermesIdentity {
    identity_id: String,
    pubkey: String,
    kind: String,
    hermes_profile_ref: String,
    reference_only: bool,
    observed_community_ids: Vec<String>,
}

#[test]
fn bundled_seeds_form_one_valid_typed_workforce_without_duplicate_pubkeys() {
    let catalog: Catalog = from_str(include_str!("../resources/workforce/role-catalog.v1.json"));
    let registry: Registry = from_str(include_str!(
        "../resources/workforce/community-registry.v1.json"
    ));
    let hermes: HermesSeed = from_str(include_str!(
        "../resources/workforce/hermes-identities.v1.json"
    ));
    assert_eq!(catalog.roles.len(), 35);
    assert_eq!(hermes.identities.len(), 9);

    let employee_identities: Vec<Value> = catalog
        .roles
        .iter()
        .map(|role| {
            json!({
                "identityId": role.identity_id,
                "pubkey": role.persona_id,
                "kind": "employee",
                "role": {
                    "roleId": role.role_id,
                    "department": role.department,
                    "useWhen": role.use_when,
                    "requiredInputs": role.required_inputs,
                    "expectedOutputs": role.expected_outputs,
                    "generalPrompt": role.general_prompt,
                    "defaultModel": role.default_model,
                    "approvalBoundary": role.approval_boundary,
                    "catalogVersion": catalog.catalog_version,
                    "lastVerifiedAt": "2026-08-19T00:00:00Z"
                }
            })
        })
        .collect();
    let hermes_identities: Vec<Value> = hermes
        .identities
        .iter()
        .map(|identity| {
            assert_eq!(identity.kind, "hermes");
            assert!(identity.reference_only);
            json!({
                "identityId": identity.identity_id,
                "pubkey": identity.pubkey,
                "kind": "hermes",
                "hermesProfileRef": identity.hermes_profile_ref
            })
        })
        .collect();
    let all_identity_ids: Vec<String> = employee_identities
        .iter()
        .chain(hermes_identities.iter())
        .map(|identity| identity["identityId"].as_str().unwrap().to_string())
        .collect();
    let observed_hermes_by_company: BTreeMap<&str, Vec<&str>> = registry
        .companies
        .iter()
        .map(|company| {
            (
                company.company_id.as_str(),
                hermes
                    .identities
                    .iter()
                    .filter(|identity| {
                        identity
                            .observed_community_ids
                            .contains(&company.company_id)
                    })
                    .map(|identity| identity.identity_id.as_str())
                    .collect(),
            )
        })
        .collect();
    let communities: Vec<Value> = registry
        .companies
        .iter()
        .map(|company| {
            let is_existing = company.relay_url.is_some();
            let membership_ids: Vec<&str> = if is_existing {
                all_identity_ids.iter().map(String::as_str).collect()
            } else {
                Vec::new()
            };
            if is_existing {
                assert_eq!(
                    observed_hermes_by_company[company.company_id.as_str()].len(),
                    9
                );
            }
            json!({
                "companyId": company.company_id,
                "displayName": company.display_name,
                "relayUrl": company.relay_url,
                "contextRef": company.context_ref,
                "defaultLanguage": company.default_language,
                "owners": [], "approvers": [],
                "rolloutState": company.rollout_state,
                "modelPolicy": {},
                "memberships": membership_ids.into_iter().map(|identity_id| json!({
                    "identityId": identity_id,
                    "enabled": false,
                    "startOnAppLaunch": false
                })).collect::<Vec<_>>()
            })
        })
        .collect();
    let store: WorkforceStore = serde_json::from_value(json!({
        "schemaVersion": 1,
        "revision": 0,
        "identities": employee_identities.into_iter().chain(hermes_identities).collect::<Vec<_>>(),
        "communities": communities
    }))
    .expect("typed workforce seed");
    store.validate().expect("valid workforce seed");
    assert_eq!(store.identities.len(), 44);
    let pubkeys: HashSet<String> = store
        .identities
        .iter()
        .map(|identity| identity.pubkey.to_lowercase())
        .collect();
    assert_eq!(pubkeys.len(), 44);

    for company_id in ["damen", "dsry"] {
        let company = store
            .communities
            .iter()
            .find(|company| company.company_id == company_id)
            .unwrap();
        assert!(company.relay_url.is_none());
        assert!(company.memberships.is_empty());
    }
}

#[test]
fn every_context_is_typed_empty_of_unapproved_runtime_facts_and_has_review_items() {
    for (company_id, source) in [
        (
            "thommacclabs",
            include_str!("../resources/workforce/contexts/thommacclabs.v1.json"),
        ),
        (
            "totaltools",
            include_str!("../resources/workforce/contexts/totaltools.v1.json"),
        ),
        (
            "vanderhilst",
            include_str!("../resources/workforce/contexts/vanderhilst.v1.json"),
        ),
        (
            "damen",
            include_str!("../resources/workforce/contexts/damen.v1.json"),
        ),
        (
            "dsry",
            include_str!("../resources/workforce/contexts/dsry.v1.json"),
        ),
    ] {
        let typed: CompanyContext = from_str(source);
        let raw: Value = from_str(source);
        assert_eq!(typed.company_id, company_id);
        assert_eq!(typed.approved_facts().count(), 0);
        assert!(!raw["reviewItems"].as_array().unwrap().is_empty());
    }
}

#[test]
fn hermes_seed_contains_references_only_and_no_runtime_configuration() {
    let source = include_str!("../resources/workforce/hermes-identities.v1.json");
    let value: Value = from_str(source);
    for forbidden in [
        "nsec",
        "privateKey",
        "systemPrompt",
        "generalPrompt",
        "model",
        "provider",
        "tools",
        "envVars",
    ] {
        assert!(
            !contains_key(&value, forbidden),
            "forbidden field {forbidden}"
        );
    }
}

fn contains_key(value: &Value, forbidden: &str) -> bool {
    match value {
        Value::Object(object) => {
            object.contains_key(forbidden)
                || object.values().any(|value| contains_key(value, forbidden))
        }
        Value::Array(values) => values.iter().any(|value| contains_key(value, forbidden)),
        _ => false,
    }
}

fn from_str<T: serde::de::DeserializeOwned>(source: &str) -> T {
    serde_json::from_str(source).unwrap()
}

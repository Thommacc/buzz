use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

pub const WORKFORCE_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkforceStore {
    pub schema_version: u32,
    #[serde(default)]
    pub revision: u64,
    #[serde(default)]
    pub identities: Vec<WorkforceIdentity>,
    #[serde(default)]
    pub communities: Vec<CommunityRecord>,
}

impl Default for WorkforceStore {
    fn default() -> Self {
        Self {
            schema_version: WORKFORCE_SCHEMA_VERSION,
            revision: 0,
            identities: Vec::new(),
            communities: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkforceIdentityKind {
    Employee,
    Hermes,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkforceIdentity {
    pub identity_id: String,
    pub pubkey: String,
    pub kind: WorkforceIdentityKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<EmployeeRole>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hermes_profile_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EmployeeRole {
    pub role_id: String,
    pub department: String,
    pub use_when: String,
    #[serde(default)]
    pub required_inputs: Vec<String>,
    #[serde(default)]
    pub expected_outputs: Vec<String>,
    pub general_prompt: String,
    pub default_model: ModelRoute,
    pub approval_boundary: String,
    pub catalog_version: u32,
    pub last_verified_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct ModelRoute {
    pub provider: String,
    pub model: String,
    pub purpose_label: String,
    pub approved: bool,
    pub health: ModelRouteHealth,
    pub last_health_check_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ModelRouteHealth {
    Healthy,
    Unhealthy,
    Unknown,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModelPolicy {
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub role_overrides: BTreeMap<String, ModelRoute>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RolloutState {
    Draft,
    Canary,
    Active,
    Disabled,
}

impl RolloutState {
    pub fn permits_runtime(&self) -> bool {
        matches!(self, Self::Canary | Self::Active)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CommunityRecord {
    pub company_id: String,
    pub display_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relay_url: Option<String>,
    pub context_ref: String,
    pub default_language: String,
    #[serde(default)]
    pub owners: Vec<String>,
    #[serde(default)]
    pub approvers: Vec<String>,
    pub rollout_state: RolloutState,
    #[serde(default)]
    pub model_policy: ModelPolicy,
    #[serde(default)]
    pub memberships: Vec<CommunityMembership>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CommunityMembership {
    pub identity_id: String,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub start_on_app_launch: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CompanyContext {
    pub company_id: String,
    pub version: u64,
    #[serde(default)]
    pub revision: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approved_at: Option<String>,
    #[serde(default)]
    pub facts: Vec<ContextFact>,
    #[serde(default)]
    pub documents: Vec<ContextDocument>,
    #[serde(default)]
    pub templates: BTreeMap<String, String>,
}

impl CompanyContext {
    pub fn approved_facts(&self) -> impl Iterator<Item = &ContextFact> {
        self.facts
            .iter()
            .filter(|fact| matches!(fact.status, ContextFactStatus::Approved { .. }))
    }

    pub fn propose_fact(&mut self, fact: ContextFact) -> Result<(), String> {
        if !matches!(fact.status, ContextFactStatus::Proposed) {
            return Err("new context facts must start as proposed".into());
        }
        if fact.confidence_basis_points > 10_000 {
            return Err("fact confidence cannot exceed 10000 basis points".into());
        }
        if self
            .facts
            .iter()
            .any(|existing| existing.fact_id == fact.fact_id)
        {
            return Err(format!("fact {:?} already exists", fact.fact_id));
        }
        self.facts.push(fact);
        self.revision = self.revision.saturating_add(1);
        Ok(())
    }

    pub fn approve_fact(
        &mut self,
        fact_id: &str,
        reviewer: &str,
        reviewed_at: &str,
    ) -> Result<(), String> {
        let fact = self.fact_mut(fact_id)?;
        if !matches!(fact.status, ContextFactStatus::Proposed) {
            return Err(format!("fact {fact_id:?} is not proposed"));
        }
        fact.status = ContextFactStatus::Approved {
            reviewer: reviewer.into(),
            reviewed_at: reviewed_at.into(),
        };
        self.version = self.version.saturating_add(1);
        self.revision = self.revision.saturating_add(1);
        self.approved_at = Some(reviewed_at.into());
        Ok(())
    }

    pub fn reject_fact(
        &mut self,
        fact_id: &str,
        reviewer: &str,
        reviewed_at: &str,
        reason: &str,
    ) -> Result<(), String> {
        let fact = self.fact_mut(fact_id)?;
        if !matches!(fact.status, ContextFactStatus::Proposed) {
            return Err(format!("fact {fact_id:?} is not proposed"));
        }
        fact.status = ContextFactStatus::Rejected {
            reviewer: reviewer.into(),
            reviewed_at: reviewed_at.into(),
            reason: reason.into(),
        };
        self.revision = self.revision.saturating_add(1);
        Ok(())
    }

    pub fn supersede_fact(
        &mut self,
        fact_id: &str,
        replacement_fact_id: &str,
        superseded_at: &str,
    ) -> Result<(), String> {
        let replacement_is_approved = self.facts.iter().any(|fact| {
            fact.fact_id == replacement_fact_id
                && matches!(fact.status, ContextFactStatus::Approved { .. })
        });
        if !replacement_is_approved {
            return Err(format!(
                "replacement fact {replacement_fact_id:?} is not approved"
            ));
        }
        let fact = self.fact_mut(fact_id)?;
        if !matches!(fact.status, ContextFactStatus::Approved { .. }) {
            return Err(format!("fact {fact_id:?} is not approved"));
        }
        fact.status = ContextFactStatus::Superseded {
            replacement_fact_id: replacement_fact_id.into(),
            superseded_at: superseded_at.into(),
        };
        self.version = self.version.saturating_add(1);
        self.revision = self.revision.saturating_add(1);
        self.approved_at = Some(superseded_at.into());
        Ok(())
    }

    fn fact_mut(&mut self, fact_id: &str) -> Result<&mut ContextFact, String> {
        self.facts
            .iter_mut()
            .find(|fact| fact.fact_id == fact_id)
            .ok_or_else(|| format!("fact {fact_id:?} not found"))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ContextFact {
    pub fact_id: String,
    pub statement: String,
    pub source: String,
    pub proposed_at: String,
    pub confidence_basis_points: u16,
    pub status: ContextFactStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", tag = "status")]
pub enum ContextFactStatus {
    Proposed,
    Approved {
        reviewer: String,
        reviewed_at: String,
    },
    Rejected {
        reviewer: String,
        reviewed_at: String,
        reason: String,
    },
    Superseded {
        replacement_fact_id: String,
        superseded_at: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ContextDocument {
    pub document_id: String,
    pub title: String,
    pub source_ref: String,
    pub approved_version: String,
}

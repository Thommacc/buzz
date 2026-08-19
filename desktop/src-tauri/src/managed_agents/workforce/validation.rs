use std::collections::{HashMap, HashSet};
use std::fmt;

use super::{
    CommunityRecord, RolloutState, WorkforceIdentityKind, WorkforceStore, WORKFORCE_SCHEMA_VERSION,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkforceValidationError {
    UnsupportedSchemaVersion {
        expected: u32,
        actual: u32,
    },
    DuplicateCompanyId(String),
    DuplicateIdentityId(String),
    DuplicatePubkey(String),
    DuplicateRelayUrl {
        relay_url: String,
    },
    DuplicateMembership {
        company_id: String,
        identity_id: String,
    },
    InvalidRelayUrl {
        company_id: String,
        reason: String,
    },
    InvalidPubkey(String),
    InvalidEmployeeIdentity(String),
    InvalidHermesIdentity(String),
    MissingActiveRelay(String),
    UnknownIdentityReference {
        company_id: String,
        identity_id: String,
    },
    UnknownRoleReference {
        company_id: String,
        role_id: String,
    },
}

impl fmt::Display for WorkforceValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for WorkforceValidationError {}

pub struct ValidatedWorkforce<'a> {
    communities_by_relay: HashMap<String, &'a CommunityRecord>,
}

impl WorkforceStore {
    pub fn validate(&self) -> Result<ValidatedWorkforce<'_>, WorkforceValidationError> {
        if self.schema_version != WORKFORCE_SCHEMA_VERSION {
            return Err(WorkforceValidationError::UnsupportedSchemaVersion {
                expected: WORKFORCE_SCHEMA_VERSION,
                actual: self.schema_version,
            });
        }

        let mut identity_ids = HashSet::new();
        let mut pubkeys = HashSet::new();
        let mut role_ids = HashSet::new();
        for identity in &self.identities {
            if !identity_ids.insert(identity.identity_id.as_str()) {
                return Err(WorkforceValidationError::DuplicateIdentityId(
                    identity.identity_id.clone(),
                ));
            }
            if identity.pubkey.len() != 64
                || !identity.pubkey.bytes().all(|byte| byte.is_ascii_hexdigit())
            {
                return Err(WorkforceValidationError::InvalidPubkey(
                    identity.identity_id.clone(),
                ));
            }
            if !pubkeys.insert(identity.pubkey.to_ascii_lowercase()) {
                return Err(WorkforceValidationError::DuplicatePubkey(
                    identity.pubkey.clone(),
                ));
            }
            match identity.kind {
                WorkforceIdentityKind::Employee => {
                    let role = identity.role.as_ref().ok_or_else(|| {
                        WorkforceValidationError::InvalidEmployeeIdentity(
                            identity.identity_id.clone(),
                        )
                    })?;
                    if identity.hermes_profile_ref.is_some()
                        || role.role_id.trim().is_empty()
                        || !role_ids.insert(role.role_id.as_str())
                    {
                        return Err(WorkforceValidationError::InvalidEmployeeIdentity(
                            identity.identity_id.clone(),
                        ));
                    }
                }
                WorkforceIdentityKind::Hermes => {
                    if identity.role.is_some()
                        || identity
                            .hermes_profile_ref
                            .as_deref()
                            .is_none_or(|profile_ref| profile_ref.trim().is_empty())
                    {
                        return Err(WorkforceValidationError::InvalidHermesIdentity(
                            identity.identity_id.clone(),
                        ));
                    }
                }
            }
        }

        let mut company_ids = HashSet::new();
        let mut communities_by_relay = HashMap::new();
        for community in &self.communities {
            if !company_ids.insert(community.company_id.as_str()) {
                return Err(WorkforceValidationError::DuplicateCompanyId(
                    community.company_id.clone(),
                ));
            }
            let canonical_relay = match community.relay_url.as_deref() {
                Some(relay_url) => Some(
                    buzz_core_pkg::relay::normalize_relay_url(relay_url).map_err(|error| {
                        WorkforceValidationError::InvalidRelayUrl {
                            company_id: community.company_id.clone(),
                            reason: error.to_string(),
                        }
                    })?,
                ),
                None => None,
            };
            if community.rollout_state != RolloutState::Draft && canonical_relay.is_none() {
                return Err(WorkforceValidationError::MissingActiveRelay(
                    community.company_id.clone(),
                ));
            }
            if let Some(relay_url) = canonical_relay {
                if communities_by_relay
                    .insert(relay_url.clone(), community)
                    .is_some()
                {
                    return Err(WorkforceValidationError::DuplicateRelayUrl { relay_url });
                }
            }

            let mut membership_ids = HashSet::new();
            for membership in &community.memberships {
                if !membership_ids.insert(membership.identity_id.as_str()) {
                    return Err(WorkforceValidationError::DuplicateMembership {
                        company_id: community.company_id.clone(),
                        identity_id: membership.identity_id.clone(),
                    });
                }
                if !identity_ids.contains(membership.identity_id.as_str()) {
                    return Err(WorkforceValidationError::UnknownIdentityReference {
                        company_id: community.company_id.clone(),
                        identity_id: membership.identity_id.clone(),
                    });
                }
            }
            for role_id in community.model_policy.role_overrides.keys() {
                if !role_ids.contains(role_id.as_str()) {
                    return Err(WorkforceValidationError::UnknownRoleReference {
                        company_id: community.company_id.clone(),
                        role_id: role_id.clone(),
                    });
                }
            }
        }

        Ok(ValidatedWorkforce {
            communities_by_relay,
        })
    }
}

impl ValidatedWorkforce<'_> {
    pub fn community_for_relay(&self, relay_url: &str) -> Option<&CommunityRecord> {
        let canonical = buzz_core_pkg::relay::normalize_relay_url(relay_url).ok()?;
        self.communities_by_relay
            .get(&canonical)
            .copied()
            .filter(|community| community.rollout_state.permits_runtime())
    }
}

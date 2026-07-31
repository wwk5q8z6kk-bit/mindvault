//! Collaborative Spaces (ADR 010) — tenancy primitives and default-deny auth.
//!
//! Distinct from document `KnowledgeWorkspace` (ADR 008/009). Product-level
//! containers are `CollabWorkspace`; authorization is always Space-scoped.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::interoperability::ActorKind;

macro_rules! collab_string_enum {
    ($(#[$meta:meta])* pub enum $name:ident { $($variant:ident => $lit:literal),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(rename_all = "snake_case")]
        pub enum $name {
            $($variant),+
        }

        impl $name {
            pub fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $lit),+
                }
            }

            pub fn parse(value: &str) -> Result<Self, String> {
                match value {
                    $($lit => Ok(Self::$variant),)+
                    other => Err(format!(
                        "unknown {}: {other}",
                        stringify!($name)
                    )),
                }
            }
        }
    };
}

collab_string_enum! {
    /// Lifecycle for a product-level collaboration container.
    pub enum CollabWorkspaceState {
        Active => "active",
        Suspended => "suspended",
        Retired => "retired",
    }
}

collab_string_enum! {
    /// Lifecycle for a Collaborative Space.
    pub enum SpaceState {
        Active => "active",
        Archived => "archived",
        Retired => "retired",
    }
}

collab_string_enum! {
    /// Membership role inside one Space. Narrower roles never imply broader ones.
    pub enum SpaceRole {
        Owner => "owner",
        Admin => "admin",
        Member => "member",
        Viewer => "viewer",
    }
}

collab_string_enum! {
    /// Membership lifecycle. Only `active` (and unexpired) grants access.
    pub enum MembershipState {
        Active => "active",
        Suspended => "suspended",
        Revoked => "revoked",
    }
}

collab_string_enum! {
    /// Owned resource kinds used for cross-Space isolation tests.
    pub enum SpaceResourceKind {
        Document => "document",
        Artifact => "artifact",
        Subscription => "subscription",
        SearchIndex => "search_index",
        Generic => "generic",
    }
}

collab_string_enum! {
    /// Operations covered by the SPACE-001 authorization matrix.
    pub enum SpaceOperation {
        Read => "read",
        Write => "write",
        Subscription => "subscription",
        Search => "search",
        Artifact => "artifact",
    }
}

/// Administrative container for Spaces (not an authorization shortcut).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CollabWorkspace {
    pub id: Uuid,
    pub slug: String,
    pub display_name: String,
    pub state: CollabWorkspaceState,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl CollabWorkspace {
    pub fn new(slug: impl Into<String>, display_name: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::now_v7(),
            slug: slug.into(),
            display_name: display_name.into(),
            state: CollabWorkspaceState::Active,
            created_at: now,
            updated_at: now,
        }
    }
}

/// Collaborative Space — primary authorization and ownership boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Space {
    pub id: Uuid,
    pub collab_workspace_id: Uuid,
    pub slug: String,
    pub display_name: String,
    pub state: SpaceState,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Space {
    pub fn new(
        collab_workspace_id: Uuid,
        slug: impl Into<String>,
        display_name: impl Into<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::now_v7(),
            collab_workspace_id,
            slug: slug.into(),
            display_name: display_name.into(),
            state: SpaceState::Active,
            created_at: now,
            updated_at: now,
        }
    }
}

/// Active (or historical) membership of one Actor in one Space.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpaceMembership {
    pub id: Uuid,
    pub space_id: Uuid,
    pub principal_id: Uuid,
    pub actor_kind: ActorKind,
    pub role: SpaceRole,
    pub state: MembershipState,
    pub granted_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

impl SpaceMembership {
    pub fn new(
        space_id: Uuid,
        principal_id: Uuid,
        actor_kind: ActorKind,
        role: SpaceRole,
    ) -> Self {
        Self {
            id: Uuid::now_v7(),
            space_id,
            principal_id,
            actor_kind,
            role,
            state: MembershipState::Active,
            granted_at: Utc::now(),
            expires_at: None,
        }
    }

    pub fn is_effectively_active(&self, now: DateTime<Utc>) -> bool {
        if self.state != MembershipState::Active {
            return false;
        }
        match self.expires_at {
            Some(expires) => expires > now,
            None => true,
        }
    }
}

/// Resource owned by a Space (tenancy envelope for isolation checks).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpaceResource {
    pub id: Uuid,
    pub space_id: Uuid,
    pub resource_kind: SpaceResourceKind,
    pub resource_key: String,
    pub owner_principal_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

impl SpaceResource {
    pub fn new(
        space_id: Uuid,
        resource_kind: SpaceResourceKind,
        resource_key: impl Into<String>,
        owner_principal_id: Option<Uuid>,
    ) -> Self {
        Self {
            id: Uuid::now_v7(),
            space_id,
            resource_kind,
            resource_key: resource_key.into(),
            owner_principal_id,
            created_at: Utc::now(),
        }
    }
}

/// Decision returned by Space authorization (always fail-closed).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpaceAuthDecision {
    Allow,
    Deny(&'static str),
}

/// Evaluate Space authorization. Default-deny for missing/inactive/cross-Space
/// membership. Role capability is secondary to tenancy isolation.
pub fn authorize_space_operation(
    membership: Option<&SpaceMembership>,
    resource_space_id: Uuid,
    operation: SpaceOperation,
    now: DateTime<Utc>,
) -> SpaceAuthDecision {
    let Some(membership) = membership else {
        return SpaceAuthDecision::Deny("no space membership");
    };
    if !membership.is_effectively_active(now) {
        return SpaceAuthDecision::Deny("membership is not active");
    }
    if membership.space_id != resource_space_id {
        return SpaceAuthDecision::Deny("cross-space access denied");
    }
    if role_permits(membership.role, operation) {
        SpaceAuthDecision::Allow
    } else {
        SpaceAuthDecision::Deny("role does not permit operation")
    }
}

fn role_permits(role: SpaceRole, operation: SpaceOperation) -> bool {
    match role {
        SpaceRole::Owner | SpaceRole::Admin | SpaceRole::Member => true,
        SpaceRole::Viewer => matches!(
            operation,
            SpaceOperation::Read
                | SpaceOperation::Subscription
                | SpaceOperation::Search
                | SpaceOperation::Artifact
        ),
    }
}

#[cfg(test)]
mod space_authorization_matrix {
    use super::*;

    fn principal() -> Uuid {
        Uuid::now_v7()
    }

    fn membership(space_id: Uuid, kind: ActorKind, role: SpaceRole) -> SpaceMembership {
        SpaceMembership::new(space_id, principal(), kind, role)
    }

    const OPS: [SpaceOperation; 5] = [
        SpaceOperation::Read,
        SpaceOperation::Write,
        SpaceOperation::Subscription,
        SpaceOperation::Search,
        SpaceOperation::Artifact,
    ];

    const KINDS: [ActorKind; 4] = [
        ActorKind::Human,
        ActorKind::Agent,
        ActorKind::Service,
        ActorKind::Integration,
    ];

    /// Deny-by-default: every actor kind without membership is refused every op.
    #[test]
    fn space_authorization_matrix_denies_all_kinds_without_membership() {
        let space_id = Uuid::now_v7();
        let now = Utc::now();
        for kind in KINDS {
            // Actor kind is irrelevant when there is no membership row.
            let _ = kind;
            for op in OPS {
                assert_eq!(
                    authorize_space_operation(None, space_id, op, now),
                    SpaceAuthDecision::Deny("no space membership")
                );
            }
        }
    }

    /// Cross-Space isolation: membership in A never authorizes a resource in B.
    #[test]
    fn space_authorization_matrix_isolates_cross_space_ops() {
        let space_a = Uuid::now_v7();
        let space_b = Uuid::now_v7();
        let now = Utc::now();
        for kind in KINDS {
            let m = membership(space_a, kind, SpaceRole::Owner);
            for op in OPS {
                assert_eq!(
                    authorize_space_operation(Some(&m), space_b, op, now),
                    SpaceAuthDecision::Deny("cross-space access denied"),
                    "{kind:?} owner of A must not {op:?} in B"
                );
            }
        }
    }

    #[test]
    fn space_authorization_matrix_allows_same_space_member_ops() {
        let space_id = Uuid::now_v7();
        let now = Utc::now();
        for kind in KINDS {
            let m = membership(space_id, kind, SpaceRole::Member);
            for op in OPS {
                assert_eq!(
                    authorize_space_operation(Some(&m), space_id, op, now),
                    SpaceAuthDecision::Allow,
                    "{kind:?} member must {op:?} in home space"
                );
            }
        }
    }

    #[test]
    fn space_authorization_matrix_viewer_cannot_write() {
        let space_id = Uuid::now_v7();
        let now = Utc::now();
        let m = membership(space_id, ActorKind::Human, SpaceRole::Viewer);
        assert_eq!(
            authorize_space_operation(Some(&m), space_id, SpaceOperation::Write, now),
            SpaceAuthDecision::Deny("role does not permit operation")
        );
        assert_eq!(
            authorize_space_operation(Some(&m), space_id, SpaceOperation::Read, now),
            SpaceAuthDecision::Allow
        );
    }

    #[test]
    fn space_authorization_matrix_suspended_membership_is_denied() {
        let space_id = Uuid::now_v7();
        let now = Utc::now();
        let mut m = membership(space_id, ActorKind::Agent, SpaceRole::Admin);
        m.state = MembershipState::Suspended;
        assert_eq!(
            authorize_space_operation(Some(&m), space_id, SpaceOperation::Read, now),
            SpaceAuthDecision::Deny("membership is not active")
        );
    }
}

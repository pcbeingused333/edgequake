//! Clearance values. Ranks are the ordered classification. Compartments are
//! an unordered set of vocabulary tag ids.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use uuid::Uuid;

/// `public`.
pub const RANK_PUBLIC: u8 = 1;
/// `internal`. Existing documents take this rank when enforcement is enabled.
pub const RANK_INTERNAL: u8 = 2;
/// `confidential`.
pub const RANK_CONFIDENTIAL: u8 = 3;
/// `restricted`. Highest rank a `classification_at_most` rule can grant.
pub const RANK_RESTRICTED: u8 = 4;
/// `unverified`. Above every allow rule. Steward, or the uploader, only.
pub const RANK_UNVERIFIED: u8 = 5;

/// Fixed classification names. Operators cannot reorder or delete them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Classification {
    Public,
    Internal,
    Confidential,
    Restricted,
    Unverified,
}

impl Classification {
    pub fn rank(self) -> u8 {
        match self {
            Self::Public => RANK_PUBLIC,
            Self::Internal => RANK_INTERNAL,
            Self::Confidential => RANK_CONFIDENTIAL,
            Self::Restricted => RANK_RESTRICTED,
            Self::Unverified => RANK_UNVERIFIED,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::Internal => "internal",
            Self::Confidential => "confidential",
            Self::Restricted => "restricted",
            Self::Unverified => "unverified",
        }
    }
}

/// Display name for a stored rank. `None` when the rank is not a seed value.
pub fn rank_name(rank: u8) -> Option<&'static str> {
    Classification::from_rank(rank).map(Classification::as_str)
}

/// Parse a classification name. Comparison is case-sensitive and exact.
pub fn rank_from_name(name: &str) -> Option<u8> {
    Some(match name {
        "public" => RANK_PUBLIC,
        "internal" => RANK_INTERNAL,
        "confidential" => RANK_CONFIDENTIAL,
        "restricted" => RANK_RESTRICTED,
        "unverified" => RANK_UNVERIFIED,
        _ => return None,
    })
}

impl Classification {
    fn from_rank(rank: u8) -> Option<Self> {
        Some(match rank {
            RANK_PUBLIC => Self::Public,
            RANK_INTERNAL => Self::Internal,
            RANK_CONFIDENTIAL => Self::Confidential,
            RANK_RESTRICTED => Self::Restricted,
            RANK_UNVERIFIED => Self::Unverified,
            _ => return None,
        })
    }
}

/// What the caller is trying to do with a label.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClearanceAction {
    /// Read a stored label.
    Read,
    /// Create an object that will carry this label.
    Write,
}

/// Allow or deny. There is no third state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decision {
    Allow,
    Deny,
}

impl Decision {
    pub fn is_allow(self) -> bool {
        matches!(self, Self::Allow)
    }
}

/// Rule effect. Deny is applied after every allow and wins.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Effect {
    Allow,
    Deny,
}

/// The four match kinds. `any_of` and `all_of` both add tags to the subject.
/// The read check is always compartment subset.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "match_kind", rename_all = "snake_case")]
pub enum MatchKind {
    ClassificationAtMost { rank: u8 },
    AnyOf { tag_ids: BTreeSet<Uuid> },
    AllOf { tag_ids: BTreeSet<Uuid> },
    Untagged,
}

/// One rule on a security role.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClearanceRule {
    pub effect: Effect,
    #[serde(flatten)]
    pub kind: MatchKind,
}

/// A role assignment presented to the compiler.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoleGrant {
    pub steward: bool,
    pub rules: Vec<ClearanceRule>,
}

/// The label stored on a document, copied onto child rows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectLabel {
    /// `None` is an unlabeled row. Enforcement on fails closed.
    pub rank: Option<u8>,
    pub compartments: BTreeSet<Uuid>,
    pub uploaded_by: Option<Uuid>,
}

impl ObjectLabel {
    pub fn new(rank: u8, compartments: impl IntoIterator<Item = Uuid>) -> Self {
        Self {
            rank: Some(rank),
            compartments: compartments.into_iter().collect(),
            uploaded_by: None,
        }
    }

    pub fn uploaded_by(mut self, user_id: Uuid) -> Self {
        self.uploaded_by = Some(user_id);
        self
    }
}

/// Compiled subject. Built once per request. Deny has already been subtracted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubjectClearance {
    /// While false, [`crate::clearance::allows`] allows every object.
    pub enforcement: bool,
    /// Process flag. The PEP writes the audit row. This struct does not.
    pub break_glass: bool,
    pub steward: bool,
    /// `0` when the subject has no classification grant.
    pub rank: u8,
    pub compartments: BTreeSet<Uuid>,
    /// Steward holds every vocabulary compartment except [`Self::denied_compartments`].
    pub holds_all_compartments: bool,
    pub denied_compartments: BTreeSet<Uuid>,
    /// Empty document compartments are visible when this is true.
    pub allow_empty_compartments: bool,
    pub user_id: Option<Uuid>,
}

impl SubjectClearance {
    /// Enforcement off. Lists, query, and graph keep the pre-174 behavior.
    pub fn enforcement_off() -> Self {
        Self {
            enforcement: false,
            break_glass: false,
            steward: false,
            rank: 0,
            compartments: BTreeSet::new(),
            holds_all_compartments: false,
            denied_compartments: BTreeSet::new(),
            allow_empty_compartments: true,
            user_id: None,
        }
    }

    pub fn with_user(mut self, user_id: Option<Uuid>) -> Self {
        self.user_id = user_id;
        self
    }

    pub fn with_break_glass(mut self, enabled: bool) -> Self {
        self.break_glass = enabled;
        self
    }
}

/// PEP input. Pure data. `bind_request` is the only builder that loads rules.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VisibilityScope {
    pub subject: SubjectClearance,
    pub policy_epoch: i64,
    /// Stable across two principals with the same compiled clearance.
    /// Uploader-only rows are not part of this value. Callers that cache a
    /// bundle also store the visible document ids.
    pub fingerprint: String,
}

impl VisibilityScope {
    pub fn new(subject: SubjectClearance, policy_epoch: i64) -> Self {
        let fingerprint = fingerprint_of(&subject);
        Self {
            subject,
            policy_epoch,
            fingerprint,
        }
    }

    pub fn enforcement_off() -> Self {
        Self::new(SubjectClearance::enforcement_off(), 0)
    }

    pub fn unrestricted(&self) -> bool {
        !self.subject.enforcement || self.subject.break_glass
    }
}

/// Cache-key material. Enforcement off and break-glass collapse to a
/// constant so two principals who see the whole workspace share a row.
pub fn fingerprint_of(subject: &SubjectClearance) -> String {
    if !subject.enforcement {
        return "off".to_string();
    }
    if subject.break_glass {
        return "break-glass".to_string();
    }
    let mut compartments: Vec<String> = subject.compartments.iter().map(Uuid::to_string).collect();
    compartments.sort();
    let mut denied: Vec<String> = subject
        .denied_compartments
        .iter()
        .map(Uuid::to_string)
        .collect();
    denied.sort();
    let canonical = format!(
        "s={}|r={}|h={}|a={}|c={}|d={}",
        u8::from(subject.steward),
        subject.rank,
        u8::from(subject.holds_all_compartments),
        u8::from(subject.allow_empty_compartments),
        compartments.join(","),
        denied.join(","),
    );
    format!("{:x}", md5::compute(canonical.as_bytes()))
}

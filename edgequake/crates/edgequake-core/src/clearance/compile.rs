//! Compile role grants into one [`SubjectClearance`].
//!
//! Allow rules run first. Deny rules then subtract, and subtraction wins.
//! A steward starts at unverified and holds every compartment. Deny still
//! applies to that grant.

use super::types::{
    Effect, MatchKind, RoleGrant, SubjectClearance, RANK_RESTRICTED, RANK_UNVERIFIED,
};
use std::collections::BTreeSet;
use uuid::Uuid;

/// Compile assignments. `user_id` is the uploader-exception principal.
/// Enforcement is on. Break-glass stays off until the process flag sets it.
pub fn compile_subject(grants: &[RoleGrant], user_id: Option<Uuid>) -> SubjectClearance {
    let steward = grants.iter().any(|grant| grant.steward);
    let mut rank = if steward { RANK_UNVERIFIED } else { 0 };
    let mut compartments = BTreeSet::new();
    let holds_all_compartments = steward;
    let mut denied_compartments = BTreeSet::new();
    let mut allow_empty_compartments = true;

    for grant in grants {
        for rule in &grant.rules {
            if rule.effect != Effect::Allow {
                continue;
            }
            apply_allow(
                &rule.kind,
                steward,
                &mut rank,
                &mut compartments,
                holds_all_compartments,
            );
        }
    }
    for grant in grants {
        for rule in &grant.rules {
            if rule.effect != Effect::Deny {
                continue;
            }
            apply_deny(
                &rule.kind,
                &mut rank,
                &mut compartments,
                &mut denied_compartments,
                &mut allow_empty_compartments,
                holds_all_compartments,
            );
        }
    }

    SubjectClearance {
        enforcement: true,
        break_glass: false,
        steward,
        rank,
        compartments,
        holds_all_compartments,
        denied_compartments,
        allow_empty_compartments,
        user_id,
    }
}

fn apply_allow(
    kind: &MatchKind,
    steward: bool,
    rank: &mut u8,
    compartments: &mut BTreeSet<Uuid>,
    holds_all: bool,
) {
    match kind {
        MatchKind::ClassificationAtMost { rank: granted } if !steward => {
            let capped = (*granted).min(RANK_RESTRICTED);
            *rank = (*rank).max(capped);
        }
        MatchKind::AnyOf { tag_ids } | MatchKind::AllOf { tag_ids } if !holds_all => {
            compartments.extend(tag_ids.iter().copied());
        }
        _ => {}
    }
}

fn apply_deny(
    kind: &MatchKind,
    rank: &mut u8,
    compartments: &mut BTreeSet<Uuid>,
    denied_compartments: &mut BTreeSet<Uuid>,
    allow_empty: &mut bool,
    holds_all: bool,
) {
    match kind {
        MatchKind::ClassificationAtMost { rank: ceiling } => {
            *rank = (*rank).min(ceiling.saturating_sub(1));
        }
        MatchKind::AnyOf { tag_ids } | MatchKind::AllOf { tag_ids } if holds_all => {
            denied_compartments.extend(tag_ids.iter().copied());
        }
        MatchKind::AnyOf { tag_ids } | MatchKind::AllOf { tag_ids } => {
            for tag_id in tag_ids {
                compartments.remove(tag_id);
            }
        }
        MatchKind::Untagged => *allow_empty = false,
    }
}

/// Compartment ids to store in `app.clearance_compartments`.
///
/// A steward's set is the workspace vocabulary minus denied tags. A document
/// that names a tag outside that vocabulary fails subset and stays hidden.
pub fn guc_compartment_ids(subject: &SubjectClearance, vocabulary: &[Uuid]) -> BTreeSet<Uuid> {
    if !subject.holds_all_compartments {
        return subject.compartments.clone();
    }
    vocabulary
        .iter()
        .copied()
        .filter(|tag_id| !subject.denied_compartments.contains(tag_id))
        .collect()
}

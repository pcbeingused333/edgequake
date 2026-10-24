//! `allows` is the only policy decision.
//!
//! Dominance is rank `<=` and compartment subset. Unverified is never
//! covered by dominance. A steward sees a row unless a deny already
//! lowered the rank or removed a compartment the row requires. The
//! uploader sees their own unverified row until that rank changes.

use super::types::{ClearanceAction, Decision, ObjectLabel, SubjectClearance, RANK_UNVERIFIED};

/// Read or write. Write is read of the label the object will carry.
/// Relabel is [`allows_relabel`], because it compares two labels.
pub fn allows(
    subject: &SubjectClearance,
    object: &ObjectLabel,
    action: ClearanceAction,
) -> Decision {
    match action {
        ClearanceAction::Read | ClearanceAction::Write => decide_read(subject, object),
    }
}

/// Relabel requires read of both labels. A lower rank, or a smaller
/// compartment set, also requires the steward flag.
pub fn allows_relabel(
    subject: &SubjectClearance,
    from: &ObjectLabel,
    to: &ObjectLabel,
) -> Decision {
    if !allows(subject, from, ClearanceAction::Read).is_allow() {
        return Decision::Deny;
    }
    if !allows(subject, to, ClearanceAction::Read).is_allow() {
        return Decision::Deny;
    }
    if is_downgrade(from, to) && !subject.steward {
        return Decision::Deny;
    }
    Decision::Allow
}

fn decide_read(subject: &SubjectClearance, object: &ObjectLabel) -> Decision {
    if !subject.enforcement || subject.break_glass {
        return Decision::Allow;
    }
    if steward_covers(subject, object)
        || uploader_unverified(subject, object)
        || dominance(subject, object)
    {
        return Decision::Allow;
    }
    Decision::Deny
}

fn steward_covers(subject: &SubjectClearance, object: &ObjectLabel) -> bool {
    if !subject.steward {
        return false;
    }
    let Some(rank) = object.rank else {
        return false;
    };
    if rank > subject.rank {
        return false;
    }
    compartments_cover(subject, object)
}

fn uploader_unverified(subject: &SubjectClearance, object: &ObjectLabel) -> bool {
    match (subject.user_id, object.uploaded_by, object.rank) {
        (Some(actor), Some(owner), Some(RANK_UNVERIFIED)) => actor == owner,
        _ => false,
    }
}

fn dominance(subject: &SubjectClearance, object: &ObjectLabel) -> bool {
    let Some(rank) = object.rank else {
        return false;
    };
    rank != RANK_UNVERIFIED
        && rank > 0
        && rank <= subject.rank
        && compartments_cover(subject, object)
}

fn compartments_cover(subject: &SubjectClearance, object: &ObjectLabel) -> bool {
    if object.compartments.is_empty() {
        return subject.allow_empty_compartments;
    }
    if subject.holds_all_compartments {
        return object
            .compartments
            .is_disjoint(&subject.denied_compartments);
    }
    object.compartments.is_subset(&subject.compartments)
}

fn is_downgrade(from: &ObjectLabel, to: &ObjectLabel) -> bool {
    let rank_dropped = match (from.rank, to.rank) {
        (Some(before), Some(after)) => after < before,
        _ => false,
    };
    let compartment_removed = from
        .compartments
        .iter()
        .any(|tag_id| !to.compartments.contains(tag_id));
    rank_dropped || compartment_removed
}

//! Algebra cases. These tests do not open a database.

use super::*;
use std::collections::BTreeSet;
use uuid::Uuid;

fn id(n: u128) -> Uuid {
    Uuid::from_u128(n)
}

fn tags(ids: &[u128]) -> BTreeSet<Uuid> {
    ids.iter().copied().map(id).collect()
}

fn allow_rank(rank: u8) -> ClearanceRule {
    ClearanceRule {
        effect: Effect::Allow,
        kind: MatchKind::ClassificationAtMost { rank },
    }
}

fn allow_any(ids: &[u128]) -> ClearanceRule {
    ClearanceRule {
        effect: Effect::Allow,
        kind: MatchKind::AnyOf { tag_ids: tags(ids) },
    }
}

fn deny_any(ids: &[u128]) -> ClearanceRule {
    ClearanceRule {
        effect: Effect::Deny,
        kind: MatchKind::AnyOf { tag_ids: tags(ids) },
    }
}

fn grant(steward: bool, rules: Vec<ClearanceRule>) -> RoleGrant {
    RoleGrant { steward, rules }
}

fn subject(grants: &[RoleGrant]) -> SubjectClearance {
    compile_subject(grants, None)
}

fn read(subject: &SubjectClearance, object: ObjectLabel) -> Decision {
    allows(subject, &object, ClearanceAction::Read)
}

const HR: u128 = 1;
const LEGAL: u128 = 2;
const RND: u128 = 3;
const ATLAS: u128 = 4;

#[test]
fn clearance_deny_wins() {
    let subject = subject(&[
        grant(
            false,
            vec![allow_rank(RANK_CONFIDENTIAL), allow_any(&[HR, ATLAS])],
        ),
        grant(false, vec![deny_any(&[ATLAS])]),
    ]);
    assert_eq!(
        read(&subject, ObjectLabel::new(RANK_CONFIDENTIAL, [id(HR)])),
        Decision::Allow
    );
    assert_eq!(
        read(
            &subject,
            ObjectLabel::new(RANK_CONFIDENTIAL, [id(HR), id(ATLAS)])
        ),
        Decision::Deny
    );
    assert!(!subject.compartments.contains(&id(ATLAS)));
    assert!(subject.compartments.contains(&id(HR)));
}

#[test]
fn clearance_equal_rank() {
    let subject = subject(&[grant(false, vec![allow_rank(RANK_CONFIDENTIAL)])]);
    assert_eq!(
        read(&subject, ObjectLabel::new(RANK_CONFIDENTIAL, [])),
        Decision::Allow
    );
}

#[test]
fn clearance_subject_superset() {
    let subject = subject(&[grant(
        false,
        vec![allow_any(&[HR, LEGAL]), allow_rank(RANK_CONFIDENTIAL)],
    )]);
    assert_eq!(
        read(&subject, ObjectLabel::new(RANK_CONFIDENTIAL, [id(HR)])),
        Decision::Allow
    );
}

#[test]
fn clearance_both_compartments() {
    let subject = subject(&[grant(
        false,
        vec![allow_any(&[HR]), allow_rank(RANK_CONFIDENTIAL)],
    )]);
    assert_eq!(
        read(
            &subject,
            ObjectLabel::new(RANK_CONFIDENTIAL, [id(HR), id(LEGAL)])
        ),
        Decision::Deny
    );
}

#[test]
fn clearance_rank_zero_denies() {
    let subject = subject(&[]);
    assert_eq!(subject.rank, 0);
    assert_eq!(
        read(&subject, ObjectLabel::new(RANK_INTERNAL, [])),
        Decision::Deny
    );
}

#[test]
fn clearance_unverified_is_uploader_only() {
    let uploader = id(9);
    let peer = subject(&[grant(false, vec![allow_rank(RANK_RESTRICTED)])]);
    let owner = compile_subject(&[], Some(uploader));
    let doc = ObjectLabel::new(RANK_UNVERIFIED, []).uploaded_by(uploader);
    assert_eq!(read(&owner, doc.clone()), Decision::Allow);
    assert_eq!(read(&peer, doc), Decision::Deny);
}

#[test]
fn clearance_allow_cannot_grant_unverified() {
    let subject = subject(&[grant(false, vec![allow_rank(RANK_UNVERIFIED)])]);
    assert_eq!(subject.rank, RANK_RESTRICTED);
    assert_eq!(
        read(&subject, ObjectLabel::new(RANK_UNVERIFIED, [])),
        Decision::Deny
    );
}

#[test]
fn clearance_steward_sees_unverified_until_deny() {
    let steward = subject(&[grant(true, vec![allow_rank(RANK_RESTRICTED)])]);
    assert_eq!(steward.rank, RANK_UNVERIFIED);
    assert!(steward.holds_all_compartments);
    assert_eq!(
        read(&steward, ObjectLabel::new(RANK_UNVERIFIED, [])),
        Decision::Allow
    );
    assert_eq!(
        read(&steward, ObjectLabel::new(RANK_CONFIDENTIAL, [id(HR)])),
        Decision::Allow
    );

    let denied = subject(&[grant(
        true,
        vec![deny_any(&[ATLAS]), allow_rank(RANK_RESTRICTED)],
    )]);
    assert_eq!(
        read(
            &denied,
            ObjectLabel::new(RANK_CONFIDENTIAL, [id(HR), id(ATLAS)])
        ),
        Decision::Deny
    );
    assert_eq!(
        read(&denied, ObjectLabel::new(RANK_CONFIDENTIAL, [id(HR)])),
        Decision::Allow
    );

    let lowered = subject(&[grant(
        true,
        vec![ClearanceRule {
            effect: Effect::Deny,
            kind: MatchKind::ClassificationAtMost {
                rank: RANK_CONFIDENTIAL,
            },
        }],
    )]);
    assert_eq!(lowered.rank, RANK_INTERNAL);
    assert_eq!(
        read(&lowered, ObjectLabel::new(RANK_UNVERIFIED, [])),
        Decision::Deny
    );
    assert_eq!(
        read(&lowered, ObjectLabel::new(RANK_RESTRICTED, [])),
        Decision::Deny
    );
}

#[test]
fn clearance_uploader_loses_body_after_raise() {
    let uploader = id(9);
    let subject = compile_subject(
        &[grant(false, vec![allow_rank(RANK_INTERNAL)])],
        Some(uploader),
    );
    let raised = ObjectLabel::new(RANK_CONFIDENTIAL, []).uploaded_by(uploader);
    assert_eq!(read(&subject, raised), Decision::Deny);
}

#[test]
fn clearance_null_rank_fails_closed() {
    let steward = subject(&[grant(true, vec![])]);
    let unlabeled = ObjectLabel {
        rank: None,
        compartments: BTreeSet::new(),
        uploaded_by: None,
    };
    assert_eq!(read(&steward, unlabeled), Decision::Deny);
}

#[test]
fn clearance_enforcement_off_and_break_glass_allow() {
    let off = SubjectClearance::enforcement_off();
    let doc = ObjectLabel::new(RANK_RESTRICTED, [id(HR)]);
    assert_eq!(read(&off, doc.clone()), Decision::Allow);
    let glass = subject(&[]).with_break_glass(true);
    assert_eq!(read(&glass, doc), Decision::Allow);
}

#[test]
fn clearance_write_follows_resulting_label() {
    let uploader = id(9);
    let actor = compile_subject(
        &[grant(
            false,
            vec![allow_rank(RANK_INTERNAL), allow_any(&[HR])],
        )],
        Some(uploader),
    );
    let unverified = ObjectLabel::new(RANK_UNVERIFIED, []).uploaded_by(uploader);
    assert_eq!(
        allows(&actor, &unverified, ClearanceAction::Write),
        Decision::Allow
    );
    let peer = compile_subject(
        &[grant(false, vec![allow_rank(RANK_INTERNAL)])],
        Some(id(8)),
    );
    assert_eq!(
        allows(&peer, &unverified, ClearanceAction::Write),
        Decision::Deny
    );
    let above = ObjectLabel::new(RANK_CONFIDENTIAL, [id(RND)]);
    assert_eq!(
        allows(&actor, &above, ClearanceAction::Write),
        Decision::Deny
    );
}

#[test]
fn clearance_downgrade_requires_steward() {
    let admin = subject(&[grant(
        false,
        vec![allow_rank(RANK_RESTRICTED), allow_any(&[HR])],
    )]);
    let from = ObjectLabel::new(RANK_CONFIDENTIAL, [id(HR)]);
    let to = ObjectLabel::new(RANK_INTERNAL, [id(HR)]);
    assert_eq!(allows_relabel(&admin, &from, &to), Decision::Deny);

    let steward = subject(&[grant(true, vec![])]);
    assert_eq!(allows_relabel(&steward, &from, &to), Decision::Allow);

    let raise = ObjectLabel::new(RANK_RESTRICTED, [id(HR)]);
    assert_eq!(allows_relabel(&admin, &from, &raise), Decision::Allow);
}

#[test]
fn clearance_worked_workspace() {
    let hr = subject(&[grant(
        false,
        vec![allow_rank(RANK_CONFIDENTIAL), allow_any(&[HR])],
    )]);
    let rnd = subject(&[grant(
        false,
        vec![allow_rank(RANK_CONFIDENTIAL), allow_any(&[RND])],
    )]);
    let hr_no_atlas = subject(&[
        grant(
            false,
            vec![allow_rank(RANK_CONFIDENTIAL), allow_any(&[HR, ATLAS])],
        ),
        grant(false, vec![deny_any(&[ATLAS])]),
    ]);

    let handbook = ObjectLabel::new(RANK_INTERNAL, []);
    let payroll = ObjectLabel::new(RANK_CONFIDENTIAL, [id(HR)]);
    let chip = ObjectLabel::new(RANK_CONFIDENTIAL, [id(RND)]);
    let payroll_legal = ObjectLabel::new(RANK_CONFIDENTIAL, [id(HR), id(LEGAL)]);
    let atlas = ObjectLabel::new(RANK_CONFIDENTIAL, [id(HR), id(ATLAS)]);

    assert_eq!(read(&hr, handbook.clone()), Decision::Allow);
    assert_eq!(read(&rnd, handbook.clone()), Decision::Allow);
    assert_eq!(read(&hr_no_atlas, handbook), Decision::Allow);

    assert_eq!(read(&hr, payroll.clone()), Decision::Allow);
    assert_eq!(read(&rnd, payroll.clone()), Decision::Deny);
    assert_eq!(read(&hr_no_atlas, payroll), Decision::Allow);

    assert_eq!(read(&hr, chip.clone()), Decision::Deny);
    assert_eq!(read(&rnd, chip), Decision::Allow);

    assert_eq!(read(&hr, payroll_legal), Decision::Deny);

    // Subset: HR does not hold atlas, so the document is hidden.
    // The deny on the third role removes a grant of atlas and hides it too.
    assert_eq!(read(&hr, atlas.clone()), Decision::Deny);
    assert_eq!(read(&rnd, atlas.clone()), Decision::Deny);
    assert_eq!(read(&hr_no_atlas, atlas), Decision::Deny);
}

#[test]
fn clearance_guc_compartments_expand_steward_vocabulary() {
    let steward = subject(&[grant(true, vec![deny_any(&[ATLAS])])]);
    let ids = guc_compartment_ids(&steward, &[id(HR), id(ATLAS), id(RND)]);
    assert_eq!(ids, tags(&[HR, RND]));
}

#[test]
fn clearance_fingerprint_ignores_user_and_collapses_off() {
    let left = compile_subject(
        &[grant(false, vec![allow_rank(RANK_INTERNAL)])],
        Some(id(1)),
    );
    let right = compile_subject(
        &[grant(false, vec![allow_rank(RANK_INTERNAL)])],
        Some(id(2)),
    );
    let a = VisibilityScope::new(left, 3);
    let b = VisibilityScope::new(right, 3);
    assert_eq!(a.fingerprint, b.fingerprint);
    assert_eq!(VisibilityScope::enforcement_off().fingerprint, "off");
}

#[test]
fn clearance_deny_untagged_hides_empty_compartments() {
    let subject = subject(&[grant(
        false,
        vec![
            allow_rank(RANK_RESTRICTED),
            allow_any(&[HR]),
            ClearanceRule {
                effect: Effect::Deny,
                kind: MatchKind::Untagged,
            },
        ],
    )]);
    assert!(!subject.allow_empty_compartments);
    assert_eq!(
        read(&subject, ObjectLabel::new(RANK_INTERNAL, [])),
        Decision::Deny
    );
    assert_eq!(
        read(&subject, ObjectLabel::new(RANK_INTERNAL, [id(HR)])),
        Decision::Allow
    );
}

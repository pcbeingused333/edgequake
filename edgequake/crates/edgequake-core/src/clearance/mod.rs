//! Document clearance (SPEC-174).
//!
//! One pure decision. Callers compile a [`SubjectClearance`] from role
//! grants, then ask [`allows`]. This module does not open a database or
//! read an HTTP request.

mod compile;
mod decide;
mod types;

pub use compile::{compile_subject, guc_compartment_ids};
pub use decide::{allows, allows_relabel};
#[cfg(test)]
mod tests;

pub use types::{
    rank_from_name, rank_name, Classification, ClearanceAction, ClearanceRule, Decision, Effect,
    MatchKind, ObjectLabel, RoleGrant, SubjectClearance, VisibilityScope, RANK_CONFIDENTIAL,
    RANK_INTERNAL, RANK_PUBLIC, RANK_RESTRICTED, RANK_UNVERIFIED,
};

//! Eligibility rules (SPEC-175 `07-data-and-db-contract.md` "Reads").
//!
//! ```text
//! candidates = catalog where capability holds, registered, not probe-failed
//! if ceiling rows exist for usage:  keep pairs with an enabled ceiling row
//! if narrowing rows exist for usage: drop pairs with a disabled narrowing row
//! ```
//!
//! A narrowing row cannot add a pair the ceiling omitted, because the ceiling step
//! runs first. [`widening_rows`] reports those writes so the API can reject them.

use super::{CatalogEntry, PolicyMode, PolicyRow};
use crate::usage_resolution::Usage;

/// Whether the tenant ceiling constrains `usage`.
pub fn is_constrained(ceiling: &[PolicyRow], usage: Usage) -> bool {
    ceiling.iter().any(|row| row.usage == usage)
}

/// Tenant-level mode: `constrained` when the ceiling has any row. Shared by every view.
pub fn tenant_policy_mode(ceiling: &[PolicyRow]) -> PolicyMode {
    if ceiling.is_empty() {
        PolicyMode::Unconstrained
    } else {
        PolicyMode::Constrained
    }
}

/// Mode for one usage, as reported to clients.
pub fn policy_mode(ceiling: &[PolicyRow], usage: Usage) -> PolicyMode {
    if is_constrained(ceiling, usage) {
        PolicyMode::Constrained
    } else {
        PolicyMode::Unconstrained
    }
}

/// Models a member may pick for `usage`, in catalog order.
pub fn eligible_models<'a>(
    usage: Usage,
    catalog: &'a [CatalogEntry],
    ceiling: &[PolicyRow],
    narrowing: &[PolicyRow],
) -> Vec<&'a CatalogEntry> {
    let usage_ceiling = rows_for(ceiling, usage);
    let usage_narrowing = rows_for(narrowing, usage);
    catalog
        .iter()
        .filter(|entry| entry.facts.registered)
        .filter(|entry| entry.facts.supports(usage))
        .filter(|entry| entry.facts.probe != Some(false))
        .filter(|entry| within_ceiling(entry, &usage_ceiling))
        .filter(|entry| not_narrowed_out(entry, &usage_narrowing))
        .collect()
}

/// Enabled narrowing rows that name a pair the ceiling does not allow for that usage.
///
/// Only meaningful when the ceiling constrains `usage`. An unconstrained usage has no
/// ceiling to widen. The API rejects a write when this list is not empty.
pub fn widening_rows<'a>(ceiling: &[PolicyRow], narrowing: &'a [PolicyRow]) -> Vec<&'a PolicyRow> {
    narrowing
        .iter()
        .filter(|row| row.enabled)
        .filter(|row| is_constrained(ceiling, row.usage))
        .filter(|row| !ceiling_allows(ceiling, row))
        .collect()
}

/// Usages named by `written` that no model could serve once `ceiling` and `narrowing`
/// are applied. The API refuses a write that returns a non-empty list, so a save can
/// never leave members with nothing to pick. Usages not named by `written` are ignored.
pub fn usages_left_empty(
    written: &[PolicyRow],
    catalog: &[CatalogEntry],
    ceiling: &[PolicyRow],
    narrowing: &[PolicyRow],
) -> Vec<Usage> {
    let mut named: Vec<Usage> = written.iter().map(|row| row.usage).collect();
    named.sort_by_key(|usage| usage.as_str());
    named.dedup();
    named
        .into_iter()
        .filter(|usage| eligible_models(*usage, catalog, ceiling, narrowing).is_empty())
        .collect()
}

fn rows_for(rows: &[PolicyRow], usage: Usage) -> Vec<&PolicyRow> {
    rows.iter().filter(|row| row.usage == usage).collect()
}

/// With no ceiling rows for the usage, every candidate passes. Otherwise an enabled
/// ceiling row for the pair is required.
fn within_ceiling(entry: &CatalogEntry, usage_ceiling: &[&PolicyRow]) -> bool {
    if usage_ceiling.is_empty() {
        return true;
    }
    usage_ceiling
        .iter()
        .any(|row| row.enabled && row.names(entry))
}

/// A disabled narrowing row removes the pair. A pair with no narrowing row, or with an
/// enabled one, stays.
fn not_narrowed_out(entry: &CatalogEntry, usage_narrowing: &[&PolicyRow]) -> bool {
    !usage_narrowing
        .iter()
        .any(|row| !row.enabled && row.names(entry))
}

fn ceiling_allows(ceiling: &[PolicyRow], narrowing: &PolicyRow) -> bool {
    ceiling.iter().any(|row| {
        row.enabled
            && row.usage == narrowing.usage
            && row.provider == narrowing.provider
            && row.model == narrowing.model
    })
}

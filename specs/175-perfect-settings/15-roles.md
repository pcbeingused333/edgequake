# 15 — Roles (Super Admin, Workspace Admin, Platform Admin)

Parent: [README](README.md) · Prev: [14-cross-ref](14-cross-ref.md) ·
Laws: [LAW-175-13](01-first-principles.md) · [LAW-175-17](01-first-principles.md)

No new role enum. The names on the shell are labels for rows that already
exist. GitHub separates an organization role from a repository role. Slack
Enterprise Grid separates an org admin from a workspace admin. Vercel shows
the team role in the switcher. Stripe lets only the top account role change
who holds that role. This file is that split for EdgeQuake settings.

## Three axes

| Product name | Stored as | Sees |
|--------------|-----------|------|
| Super Admin | `memberships.workspace_id` is null and `MembershipRole` is `owner` or `admin` | Organization, and every workspace in the tenant |
| Workspace Admin | `workspace_id` is that workspace and the role is `owner` or `admin` | That workspace's Models, Ingestion, Access, and narrowing |
| Platform Admin | `edgequake_auth::Role::Admin` | Deployment. Support view of a tenant. Not called Super Admin |

A member or readonly membership edits nothing in Workspace or Organization.

Symbols:

- Scope of a grant: `Membership.workspace_id` and `applies_to` in
  [membership.rs](../../edgequake/crates/edgequake-core/src/types/multitenancy/membership.rs).
  Null means every workspace in the tenant (`can_access_workspace`).
- Tenant manage: `get_user_role` keeps only null-`workspace_id` rows
  ([in_memory.rs](../../edgequake/crates/edgequake-core/src/workspace_service/in_memory.rs)
  around line 1014). `require_tenant_access` then requires
  `can_manage_users`
  ([tenant_access.rs](../../edgequake/crates/edgequake-api/src/handlers/workspaces/tenant_access.rs)).
- Workspace manage: `membership_role` takes the highest role that
  `applies_to` the selected workspace
  ([request_authorization.rs](../../edgequake/crates/edgequake-api/src/services/request_authorization.rs)).
  A write under `/workspaces` needs `can_manage_workspaces`.
- Platform: `require_platform_admin`. Create and delete tenant stay there.
  `is_platform_admin` returns success from `require_tenant_access` before
  the membership lookup.

A Super Admin is therefore an implicit Workspace Admin of every workspace
in the tenant. A Workspace Admin of Legal does not become a Super Admin
and does not administer Finance.

## What each role may change

| Act | Super Admin | Workspace Admin | Platform Admin |
|-----|-------------|-----------------|----------------|
| Catalog ceiling, tenant defaults | yes | no | yes, as support |
| Invite and membership roles | yes | no | yes, as support |
| Create a workspace | yes (`create_workspace` already requires `TenantAccess::Manage`) | no | yes |
| Delete the tenant | tenant-wide Owner only | no | yes (`require_platform_admin`) |
| Remove the last tenant Owner | no | no | no |
| Assign models, ingestion, labels, narrowing | every workspace | that workspace | yes, as support |
| Delete that workspace | yes | yes, if they admin it | yes |
| Connections, fleet, attribution, SSO, server quotas | no | no | yes |
| Query model inside the eligible set | yes | yes | yes |

Workspace delete stays any Workspace Admin of that workspace or any Super
Admin, because both satisfy `can_manage_workspaces` today. This pack does
not narrow that.

Owner versus Admin inside one scope:

- Tenant-wide Owner and tenant-wide Admin are both Super Admins on the chip.
- Only the Owner sees Delete tenant, and only the Owner can demote or
  remove the last tenant-wide Owner ([EC-175-38](11-edge-cases.md)).
- Workspace-scoped Owner and Admin are both Workspace Admins on the chip.
  Both may delete that workspace.

## Shells

Super Admin. Organization is present. The switcher lists every workspace.

```text
+------------------------------------------------------------------------+
| Acme                         chip: Super admin                         |
| <- Back to app                                                         |
+------------------------------------------------------------------------+
| YOU                                                                    |
|   Appearance                                                           |
| WORKSPACE  Legal                                                       |
|   Models          editable                                             |
|   Ingestion                                                            |
|   Access                                                               |
| ORGANIZATION                                                           |
|   Catalog                                                              |
|   Defaults                                                             |
|   People                                                               |
+------------------------------------------------------------------------+
| Switcher: Legal, Finance, Research                                     |
| Every workspace in this tenant.                                        |
+------------------------------------------------------------------------+
```

Workspace Admin of Legal. Organization and Deployment are absent. The
settings switcher lists only workspaces this person administers. The
app header still lists every workspace they can open, including one
where they are only a member ([EC-175-39](11-edge-cases.md)).

```text
+------------------------------------------------------------------------+
| Acme / Legal                 chip: Workspace admin                     |
| <- Back to app                                                         |
+------------------------------------------------------------------------+
| YOU                                                                    |
|   Appearance                                                           |
| WORKSPACE  Legal                                                       |
|   Models          editable                                             |
|   Ingestion                                                            |
|   Access                                                               |
+------------------------------------------------------------------------+
| Organization is absent.                                                |
| Deployment is absent.                                                  |
| Switcher: Legal only.                                                  |
+------------------------------------------------------------------------+
```

The same person opens Finance, which they do not administer. Models are
the member page ([W-09](09-ux-ui-spec.md#w-09-member-row)).

```text
+------------------------------------------------------------------------+
| Acme / Finance               chip: Workspace admin                     |
| You administer Legal, not Finance.                                     |
+------------------------------------------------------------------------+
| Extract     gpt-5-nano     inherited                                   |
| Query       gemma3         this workspace                              |
+------------------------------------------------------------------------+
| No radios. No Save.                                                    |
| Same page a member sees.                                               |
+------------------------------------------------------------------------+
```

Platform Admin. The chip never says Super admin, including while they
open a customer's Organization for support. They still cannot skip the
catalog: a disabled model stays disabled.

```text
+------------------------------------------------------------------------+
| EdgeQuake                    chip: Platform admin                      |
| DEPLOYMENT                                                             |
|   Connections                                                          |
|   Fleet defaults                                                       |
|   Observability                                                        |
+------------------------------------------------------------------------+
| Opening Acme shows Organization for support.                           |
| The chip stays Platform admin, not Super admin.                        |
+------------------------------------------------------------------------+
```

Badge order when one person holds more than one grant: Platform admin,
else Super admin, else Workspace admin, else Member. The higher badge
does not hide the lower scope. A Platform Admin who is also a Super Admin
still sees Deployment and Organization. The chip says Platform admin.

## Routes that must not lie

Policy writes do not mount under `/api/v1/settings/`. That prefix is a
`global_resource`, so `bind_request` does not apply the membership check
([request_authorization.rs](../../edgequake/crates/edgequake-api/src/services/request_authorization.rs)
`global_resource`). Tenant policy uses the tenant-manage gate. Workspace
policy uses the workspace-admin gate for that id.
[F-175-26](04-findings.md).

`get_user_role` ignoring workspace-scoped rows is the feature for the
tenant gate, not a bug to "fix" by letting a Workspace Admin edit the
ceiling. [F-175-25](04-findings.md).

## SSO and the last owner

An identity provider `max_role` cannot mint a tenant-wide Owner the role
map did not include ([SPEC-158](../158-entreprise-grade-authentication/)).
[EC-175-41](11-edge-cases.md).

`Membership::pin_manual_role` is how a Super Admin freezes a role so the
next sign-in does not overwrite it. A Workspace Admin cannot pin a
tenant-wide role.

The last tenant-wide Owner cannot be removed or demoted by another Super
Admin. The API returns 409 `last_tenant_owner`. Platform Admin does not
get a bypass for that row. [EC-175-38](11-edge-cases.md). Same rule Stripe
uses: the top role cannot orphan the account.

## Market map

| EdgeQuake | GitHub | Slack Grid | Vercel |
|-----------|--------|------------|--------|
| Platform Admin | Enterprise / appliance admin | Grid owner of the vendor install | Vercel staff, not a team role |
| Super Admin | Organization owner and admin | Org owner and org admin | Team owner |
| Workspace Admin | Repository admin | Workspace admin | Project access without team ownership |
| Member | Repository write or read | Member | Team member or viewer |

Sources: [GitHub organization roles](https://docs.github.com/en/organizations/managing-peoples-access-to-your-organization-with-roles/roles-in-an-organization),
[Slack Grid roles](https://slack.com/help/articles/360000120963-Roles-and-permissions-in-the-org-dashboard),
[Vercel team roles](https://vercel.com/docs/accounts/team-members-and-roles).

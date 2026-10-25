# Lens — SaaS

Parent: [README](../README.md) · Catalog: [06-catalog](../06-catalog.md)

## What this lens owns

[LAW-175-5](../01-first-principles.md), [LAW-175-6](../01-first-principles.md),
and [LAW-175-13](../01-first-principles.md).

A tenant's deliberate model runs on that tenant's key. The process key
is the leaf when nobody chose anything. It is not the recovery key when
a choice fails ([EC-175-04](../11-edge-cases.md)).

## Owners

| Scope | Who pays for a mistake | Who may edit |
|-------|------------------------|--------------|
| You | that browser | the user |
| Workspace | that workspace's corpus | Workspace Admin of that workspace, or a Super Admin |
| Organization | every workspace in the tenant | Super Admin |
| Deployment | every tenant on the machine | Platform Admin |

The three names, the chip, and the market map (GitHub, Slack Grid,
Vercel, Stripe's last-owner rule) are in [15-roles](../15-roles.md).
[LAW-175-17](../01-first-principles.md). The last active tenant Owner
cannot be removed ([EC-175-38](../11-edge-cases.md)). An inactive Owner
does not count, so the tenant can be recovered. The last Workspace
Admin can be removed ([EC-175-50](../11-edge-cases.md)). Binding off
does not open the ceiling ([EC-175-51](../11-edge-cases.md)).

Self-host is the same matrix. One person may hold every role. The shell
still labels the scope so a key pasted into You is impossible: You has
no key field.

## Ceiling

Default unconstrained, so existing customers do not go dark on upgrade
([LAW-175-9](../01-first-principles.md)). The first ceiling row is a
product mode change. The confirm sentence is required. A workspace may
narrow. It may not widen ([EC-175-30](../11-edge-cases.md)).

## What stays off the customer UI

Decision backend, rerank vendor, omit-temperature, API format, manuscript
DPI. Those are operator env. The Deployment page may explain them. It
does not store a second copy that can disagree with the process.

## Market bar

K2View's "N of M shown in the picker" is the catalog
([docs](https://support.k2view.com/Academy/articles/04_fabric_studio/studio_ai/06_ai_configuration_and_settings.html)).
GitHub's split between user, organization, and repository is the shell
([survey](https://github.com/onetimesecret/onetimesecret/issues/2307)).
We do not copy a billing page in this pack.

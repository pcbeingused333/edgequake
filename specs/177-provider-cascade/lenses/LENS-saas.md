# Lens — SaaS

Parent: [README](../README.md) · Stories: [03-product-spec](../03-product-spec.md)

## What this lens owns

[LAW-177-1](../01-first-principles.md), [LAW-177-3](../01-first-principles.md),
and [LAW-177-5](../01-first-principles.md). One deployment sells many
tenants. A credential is born once, on the deployment. A tenant receives
a grant. A tenant does not receive another tenant's key, and a failed
grant does not spend the platform key.

## Isolation

| Fact | Tenant A | Tenant B |
|------|----------|----------|
| Fleet connection `tenant_id` null | in the global offer if the probe passed | same offer |
| Legacy connection owned by A | `legacy: true` on A's list | absent |
| Constrained offer | A's slugs | B's slugs |
| Operator lock | process credential, every tenant | same pin |

[EC-177-08](../11-edge-cases.md) and [EC-177-22](../11-edge-cases.md) are
the isolation gates. [EC-177-06](../11-edge-cases.md) is the billing gate:
a removed provider does not fall through to `ProviderFactory::from_env`.

## Who holds the key

Platform Admin creates and rotates it. Super Admin turns the provider on
for the organization and never sees the key. Workspace Admin turns it on
for one workspace and never sees the key. A member sees model names.

`POST /connections` with a tenant header is refused so a tenant admin
session that still sends `X-Tenant-ID` cannot create a private provider
beside the fleet. [EC-177-12](../11-edge-cases.md). The connections
mutation stops sending that header. [EC-177-47](../11-edge-cases.md).

Organization → Secrets still chooses where a tenant's own secrets live.
It is not a second place to create a model provider. New provider secrets
are stored with `tenant_id` null through the deployment secret service.

## What stays shared on purpose

`EDGEQUAKE_ALLOWED_PROVIDERS` is a process filter in front of every tenant.
A slug it excludes is not offered to anyone. This pack does not turn that
variable into a per-tenant column.

An unconstrained tenant sees providers added later. That is the upgrade
path and the "new tenant" path. [EC-177-04](../11-edge-cases.md),
[EC-177-34](../11-edge-cases.md). A constrained tenant does not.
[EC-177-05](../11-edge-cases.md). The product tells the admin which mode
they are in before they save.

## Out of this lens

Per-seat pricing, invoices, and a tenant-supplied key. New BYOK is a
later pack. This one only fences the legacy rows.

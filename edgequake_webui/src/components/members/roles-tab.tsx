'use client';

import { Badge } from '@/components/ui/badge';
import { Skeleton } from '@/components/ui/skeleton';
import { useRoleCatalog } from '@/hooks/use-members';
import type { OrgRole, RoleCatalog } from '@/lib/api/edgequake/members';
import { ORG_ROLES, ROLE_ICON } from '@/lib/members/role-meta';
import { Check, Minus } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { MembersError } from './members-states';
import { useRoleDescription, useRoleLabel } from './use-role-label';

type Capability = RoleCatalog['capabilities'][number];

function groupBy(caps: Capability[]): [string, Capability[]][] {
  const map = new Map<string, Capability[]>();
  for (const c of caps) map.set(c.group, [...(map.get(c.group) ?? []), c]);
  return [...map.entries()];
}

const key = (id: string) => id.replace(/\./g, '_');

function RoleHeaderCell({ role, count }: { role: OrgRole; count?: number }) {
  const label = useRoleLabel()(role);
  const Icon = ROLE_ICON[role];
  return (
    <th scope="col" className="w-24 px-2 py-3 text-center align-bottom" data-testid={`roles-col-${role}`}>
      <div className="flex flex-col items-center gap-1">
        <Icon className="size-4 text-muted-foreground" aria-hidden />
        <span className="text-sm font-semibold">{label}</span>
        {count !== undefined && <span className="text-[0.6875rem] font-normal text-muted-foreground">{count}</span>}
      </div>
    </th>
  );
}

/** Capability matrix — generated from `GET /roles`, so it can never drift from what the API enforces. */
function CapabilityMatrix({ catalog }: { catalog: RoleCatalog }) {
  const { t } = useTranslation();
  const label = useRoleLabel();
  return (
    <div className="overflow-x-auto rounded-lg border bg-card" data-testid="roles-matrix">
      <table className="w-full min-w-[34rem] text-sm">
        <thead className="bg-muted/40">
          <tr>
            <th scope="col" className="px-4 py-3 text-left text-xs font-medium text-muted-foreground">
              {t('members.roles.capability', 'Capability')}
            </th>
            {ORG_ROLES.map((r) => (
              <RoleHeaderCell key={r} role={r} />
            ))}
          </tr>
        </thead>
        <tbody>
          {groupBy(catalog.capabilities).map(([group, caps]) => (
            <GroupRows key={group} group={group} caps={caps} roleLabel={label} />
          ))}
        </tbody>
      </table>
    </div>
  );
}

function GroupRows({ group, caps, roleLabel }: { group: string; caps: Capability[]; roleLabel: (r: OrgRole) => string }) {
  const { t } = useTranslation();
  return (
    <>
      <tr className="border-t bg-muted/20">
        <th colSpan={5} scope="colgroup" className="px-4 py-1.5 text-left text-[0.6875rem] font-semibold uppercase tracking-wide text-muted-foreground">
          {t(`members.capabilityGroups.${group.toLowerCase()}`, group)}
        </th>
      </tr>
      {caps.map((c) => (
        <tr key={c.id} className="border-t" data-testid={`cap-${c.id}`}>
          <th scope="row" className="px-4 py-2.5 text-left font-normal">
            {t(`members.capabilities.${key(c.id)}`, c.label)}
          </th>
          {ORG_ROLES.map((r) => {
            const granted = c.roles.includes(r);
            return (
              <td key={r} className="px-2 py-2.5 text-center" data-granted={granted}>
                {granted ? (
                  <Check className="mx-auto size-4 text-success" aria-label={t('members.roles.granted', '{{role}} can', { role: roleLabel(r) })} />
                ) : (
                  <Minus className="mx-auto size-4 text-muted-foreground/40" aria-label={t('members.roles.denied', '{{role}} cannot', { role: roleLabel(r) })} />
                )}
              </td>
            );
          })}
        </tr>
      ))}
    </>
  );
}

function RoleCards({ catalog }: { catalog: RoleCatalog }) {
  const label = useRoleLabel();
  const describe = useRoleDescription();
  return (
    <ul className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4" data-testid="role-cards">
      {catalog.organization_roles.map((r) => {
        const Icon = ROLE_ICON[r.id];
        return (
          <li key={r.id} className="rounded-lg border bg-card p-4" data-testid={`role-card-${r.id}`}>
            <div className="flex items-center gap-2">
              <span className="flex size-7 items-center justify-center rounded-md bg-muted">
                <Icon className="size-4" aria-hidden />
              </span>
              <span className="text-sm font-semibold">{label(r.id)}</span>
            </div>
            <p className="mt-2 text-xs leading-relaxed text-muted-foreground">{describe(r.id, r.description)}</p>
          </li>
        );
      })}
    </ul>
  );
}

function InstanceRoles({ catalog }: { catalog: RoleCatalog }) {
  const { t } = useTranslation();
  return (
    <section className="space-y-2" data-testid="instance-roles">
      <div>
        <h3 className="text-sm font-semibold">{t('members.roles.instanceTitle', 'Instance roles')}</h3>
        <p className="text-xs text-muted-foreground">
          {t('members.roles.instanceBody', 'A separate, platform-wide setting managed by instance admins. It does not replace organization roles.')}
        </p>
      </div>
      <ul className="grid gap-2 sm:grid-cols-3">
        {catalog.instance_roles.map((r) => (
          <li key={r.id} className="rounded-lg border bg-card p-3" data-testid={`instance-role-${r.id}`}>
            <div className="flex items-center justify-between gap-2">
              <span className="text-sm font-medium">{t(`members.roles.instance.${r.id}.label`, r.label)}</span>
              <Badge variant="outline" className="text-[0.625rem] tabular-nums">
                {t('members.roles.permissions', '{{count}} permissions', { count: r.permission_count })}
              </Badge>
            </div>
            <p className="mt-1 text-xs text-muted-foreground">{t(`members.roles.instance.${r.id}.description`, r.description)}</p>
          </li>
        ))}
      </ul>
    </section>
  );
}

export function RolesTab() {
  const { t } = useTranslation();
  const q = useRoleCatalog();
  if (q.isError) return <MembersError onRetry={() => q.refetch()} />;
  if (!q.data) return <Skeleton className="h-72 w-full" data-testid="roles-skeleton" />;
  return (
    <div className="space-y-6" data-testid="roles-tab">
      <p className="text-sm text-muted-foreground">
        {t('members.roles.intro', 'Every member has one role in this organization. Roles are cumulative: each one can do everything the roles below it can.')}
      </p>
      <RoleCards catalog={q.data} />
      <CapabilityMatrix catalog={q.data} />
      <p className="text-sm text-muted-foreground">
        {t(
          'members.roles.clearanceHint',
          'Who can open which documents is set on each person and under Settings → Access.',
        )}
      </p>
      <InstanceRoles catalog={q.data} />
    </div>
  );
}

'use client';

import { SettingsShell } from '@/components/settings/shell/settings-shell';
import { useParams } from 'next/navigation';

/**
 * SPEC-175 — `/w/{slug}/settings/*` deeplinks render the same shell as `/settings`
 * (W-31). Models is the deeplink's landing page; no redirect to /workspace.
 */
export default function WorkspaceSettingsLayout() {
  const params = useParams<{ slug: string }>();
  return <SettingsShell workspaceSlug={params.slug} />;
}

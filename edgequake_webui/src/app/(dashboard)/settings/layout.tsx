import { SettingsShell } from '@/components/settings/shell/settings-shell';

/**
 * SPEC-175 — one shell for every `/settings/*` route. The shell reads the section
 * from the pathname, so the route pages below only exist to make the paths valid.
 */
export default function SettingsLayout() {
  return <SettingsShell />;
}

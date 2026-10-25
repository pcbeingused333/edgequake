'use client';

import { rememberAppPath } from '@/lib/settings/return-path';
import { usePathname } from 'next/navigation';
import { useEffect } from 'react';

/**
 * SPEC-175 — records the last non-settings page so the settings shell's "Back to app"
 * returns there. Renders nothing. Mounted once in the root layout.
 */
export function RememberAppPath() {
  const pathname = usePathname();
  useEffect(() => {
    if (pathname) rememberAppPath(pathname);
  }, [pathname]);
  return null;
}

import { test, expect } from '@playwright/test';
import { skipUnlessLiveStack } from './helpers/live-stack';

test.describe('SPEC-163 connections UI', () => {
  test('settings shows Connections card', async ({ page }) => {
    skipUnlessLiveStack();
    await page.goto('/settings');
    await expect(page.getByText('Connections', { exact: true })).toBeVisible();
    await expect(page.getByRole('button', { name: /Test connection/i })).toBeVisible();
    await expect(page.getByTestId('model-roles-card')).toBeVisible();
    await expect(page.getByTestId('provider-status-hub')).toBeVisible();
  });
});

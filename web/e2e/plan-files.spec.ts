import { readFile } from 'node:fs/promises';
import { expect, itemCount, openMenu, tapPlanMission, test } from './fixtures';
import type { Page } from '@playwright/test';

test.describe.configure({ timeout: 120_000 });

const planMenu = async (page: Page, item: string) => {
  await page.getByRole('button', { name: 'Plan menu' }).click();
  await page.getByRole('menuitem', { name: item }).click();
};

test('save as downloads the plan and open plan loads it back', async ({ app }, testInfo) => {
  await openMenu(app, 'Plan');
  await tapPlanMission(app);
  const saving = app.waitForEvent('download');
  await planMenu(app, 'Save as…');
  const naming = app.getByRole('dialog').filter({ hasText: 'Save plan as' });
  await naming.getByRole('textbox', { name: 'File name' }).fill('North field');
  await naming.getByRole('button', { name: 'Save', exact: true }).click();
  const saved = await saving;
  expect(saved.suggestedFilename(), 'Save as asks for a name instead of repeating Save').toBe('North field.plan');
  const file = testInfo.outputPath(saved.suggestedFilename());
  await saved.saveAs(file);
  const plan = JSON.parse(await readFile(file, 'utf8')) as { fileType: string; mission: { items: unknown[] } };
  expect(plan.fileType).toBe('Plan');
  expect(plan.mission.items.length).toBeGreaterThanOrEqual(4);
  await expect(app.getByText('Plan saved.')).toBeVisible();
  const exporting = app.waitForEvent('download');
  await planMenu(app, 'Export KML…');
  expect((await exporting).suggestedFilename(), 'the KML of a named plan was always called mission.kml').toBe('North field.kml');

  await planMenu(app, 'New plan…');
  await app.getByRole('dialog').filter({ hasText: 'New plan' }).getByText('Blank mission').click();
  await app.getByRole('dialog').filter({ hasText: 'Start a new plan?' }).getByRole('button', { name: 'Start new plan', exact: true }).click();
  await expect.poll(() => itemCount(app)).toBeLessThan(4);
  const choosing = app.waitForEvent('filechooser');
  await planMenu(app, 'Open plan…');
  await (await choosing).setFiles(file);
  await expect(app.getByText('Plan opened.')).toBeVisible();
  await expect.poll(() => itemCount(app)).toBeGreaterThanOrEqual(5);
});

test('export KML downloads a KML document of the mission', async ({ app }, testInfo) => {
  await openMenu(app, 'Plan');
  await tapPlanMission(app);
  const exporting = app.waitForEvent('download');
  await planMenu(app, 'Export KML…');
  const exported = await exporting;
  expect(exported.suggestedFilename()).toMatch(/\.kml$/);
  const file = testInfo.outputPath(exported.suggestedFilename());
  await exported.saveAs(file);
  expect(await readFile(file, 'utf8')).toContain('<kml');
  await expect(app.getByText('KML exported.')).toBeVisible();
});

const savedPlan = async (page: Page, file: string) => {
  const saving = page.waitForEvent('download');
  await planMenu(page, 'Save as…');
  await page.getByRole('dialog').filter({ hasText: 'Save plan as' }).getByRole('button', { name: 'Save', exact: true }).click();
  await (await saving).saveAs(file);
  await expect(page.getByText('Plan saved.')).toBeVisible();
};

const edited = async (page: Page) => {
  const box = await page.getByRole('region', { name: 'Map' }).boundingBox();
  if (box === null) throw new Error('plan map is not on screen');
  const before = await itemCount(page);
  await page.mouse.click(box.x + box.width * 0.45, box.y + box.height * 0.35);
  await expect.poll(() => itemCount(page)).toBe(before + 1);
};

test('save and open asks for the file first, then saves the changes and opens it', async ({ app }, testInfo) => {
  await openMenu(app, 'Plan');
  await tapPlanMission(app);
  const file = testInfo.outputPath('ridge.plan');
  await savedPlan(app, file);
  const saved = await itemCount(app);
  await edited(app);
  await planMenu(app, 'Open plan…');
  const choosing = app.waitForEvent('filechooser');
  const saving = app.waitForEvent('download');
  await app.getByRole('dialog').filter({ hasText: 'Open another plan?' }).getByRole('button', { name: 'Save and open' }).click();
  await (await choosing).setFiles(file);
  expect((await saving).suggestedFilename()).toBe('mission.plan');
  await expect(app.getByText('Plan opened.')).toBeVisible();
  await expect.poll(() => itemCount(app)).toBe(saved);
});

test('starting a new plan over changes can save them first', async ({ app }, testInfo) => {
  await openMenu(app, 'Plan');
  await tapPlanMission(app);
  await savedPlan(app, testInfo.outputPath('ridge.plan'));
  await edited(app);
  await planMenu(app, 'New plan…');
  await app.getByRole('dialog').filter({ hasText: 'New plan' }).getByText('Blank mission').click();
  const saving = app.waitForEvent('download');
  await app.getByRole('dialog').filter({ hasText: 'Start a new plan?' }).getByRole('button', { name: 'Save and start new plan' }).click();
  expect((await saving).suggestedFilename()).toBe('mission.plan');
  await expect.poll(() => itemCount(app)).toBeLessThan(4);
});

test('a new plan from a template is untitled, so saving it does not write over the plan it replaced', async ({ app }, testInfo) => {
  await openMenu(app, 'Plan');
  await app.getByRole('button', { name: 'Center', exact: true }).click();
  await app.getByRole('menuitem', { name: 'Coordinates…' }).click();
  const centre = app.getByRole('dialog').filter({ hasText: 'Center map on coordinate' });
  await centre.getByRole('textbox', { name: 'Latitude' }).fill('47.3977');
  await centre.getByRole('textbox', { name: 'Longitude' }).fill('8.5456');
  await centre.getByRole('button', { name: 'Center' }).click();
  await expect(app.locator('span.text-label-small').getByText(/^\d+ (m|km)$/)).toHaveText(/^\d+ m$/, { timeout: 30_000 });
  await tapPlanMission(app);
  const naming = app.waitForEvent('download');
  await planMenu(app, 'Save as…');
  const named = app.getByRole('dialog').filter({ hasText: 'Save plan as' });
  await named.getByRole('textbox', { name: 'File name' }).fill('North field');
  await named.getByRole('button', { name: 'Save', exact: true }).click();
  await (await naming).saveAs(testInfo.outputPath('North field.plan'));
  await expect(app.getByText('North field', { exact: true })).toBeVisible();
  await planMenu(app, 'New plan…');
  await app.getByRole('dialog').filter({ hasText: 'New plan' }).getByRole('button', { name: 'Survey', exact: true }).click();
  await app.getByRole('dialog').filter({ hasText: 'Start a new plan?' }).getByRole('button', { name: 'Start new plan', exact: true }).click();
  await expect(app.getByRole('button', { name: /^\d+ Survey / }), 'a survey is one numbered stop, not the run of mission items it generates').toBeVisible({ timeout: 30_000 });
  await expect(app.getByText('Untitled plan', { exact: true }), 'the survey used to keep the replaced plan’s name').toBeVisible();
  const saving = app.waitForEvent('download');
  await planMenu(app, 'Save as…');
  await app.getByRole('dialog').filter({ hasText: 'Save plan as' }).getByRole('button', { name: 'Save', exact: true }).click();
  expect((await saving).suggestedFilename()).toBe('mission.plan');
});

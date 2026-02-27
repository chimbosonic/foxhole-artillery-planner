import { test, expect } from "@playwright/test";

test.describe("Foxhole Artillery Planner", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/");
    // Wait for WASM app to hydrate - the app div with class "app" should be present
    await page.waitForSelector(".app", { timeout: 15_000 });
  });

  test("page loads with dark theme and correct title", async ({ page }) => {
    await expect(page).toHaveTitle("Foxhole Artillery Planner");

    // Verify dark theme is applied (CSS loaded)
    const body = page.locator("body");
    const bgColor = await body.evaluate(
      (el) => getComputedStyle(el).backgroundColor,
    );
    // --bg-dark: #1a1a2e → rgb(26, 26, 46)
    expect(bgColor).toBe("rgb(26, 26, 46)");
  });

  test("header and placement mode buttons render", async ({ page }) => {
    await expect(page.locator(".header h1")).toHaveText(
      "Foxhole Artillery Planner",
    );

    const buttons = page.locator(".placement-mode button");
    await expect(buttons).toHaveCount(3);
    await expect(buttons.nth(0)).toHaveText("Gun");
    await expect(buttons.nth(1)).toHaveText("Target");
    await expect(buttons.nth(2)).toHaveText("Spotter");
  });

  test("sidebar panels render", async ({ page }) => {
    // Map selector panel
    await expect(page.locator(".panel h3").first()).toBeVisible();

    // Should have Map, Weapon, Wind, Firing Solution, Plan panels
    const panelHeaders = page.locator(".panel h3");
    const texts = await panelHeaders.allTextContents();
    expect(texts).toContain("Map");
    expect(texts).toContain("Weapon");
    expect(texts).toContain("Wind");
    expect(texts).toContain("Firing Solution");
    expect(texts).toContain("Plan");
  });

  test("map image loads in container", async ({ page }) => {
    const mapContainer = page.locator(".map-container");
    await expect(mapContainer).toBeVisible();

    const img = mapContainer.locator("img");
    await expect(img).toBeVisible();

    // Image should have a src pointing to a map webp
    const src = await img.getAttribute("src");
    expect(src).toMatch(/\/static\/images\/maps\/.*\.webp/);
  });

  test("map container has correct positioning for SVG overlay", async ({
    page,
  }) => {
    const mapContainer = page.locator(".map-container");
    const position = await mapContainer.evaluate(
      (el) => getComputedStyle(el).position,
    );
    expect(position).toBe("relative");
  });

  test("SVG overlay renders on the map", async ({ page }) => {
    const mapContainer = page.locator(".map-container");
    const svgOverlay = mapContainer.locator("svg");
    await expect(svgOverlay).toBeVisible();

    // SVG should have correct viewBox matching map dimensions (1024x888)
    const viewBox = await svgOverlay.getAttribute("viewBox");
    expect(viewBox).toBe("0 0 1024 888");
  });

  test("grid lines render in SVG", async ({ page }) => {
    const svg = page.locator(".map-container svg");

    // Should have grid lines (17 col lines + 1 + 15 row lines + 1 = 34 lines minimum)
    const gridLines = svg.locator("line");
    const count = await gridLines.count();
    expect(count).toBeGreaterThanOrEqual(34);
  });

  test("grid labels render (columns A-Q and rows 1-15)", async ({ page }) => {
    const svg = page.locator(".map-container svg");
    const texts = svg.locator("text");
    const allLabels = await texts.allTextContents();

    // Check column labels
    expect(allLabels).toContain("A");
    expect(allLabels).toContain("Q");
    // Check row labels
    expect(allLabels).toContain("1");
    expect(allLabels).toContain("15");
  });

  test("map selector has options and can be changed", async ({ page }) => {
    const mapSelect = page.locator(".sidebar .panel select").first();
    await expect(mapSelect).toBeVisible();

    const options = mapSelect.locator("option");
    const count = await options.count();
    expect(count).toBeGreaterThan(1);
  });

  test("weapon selector has faction optgroups", async ({ page }) => {
    const weaponSelect = page
      .locator('.panel:has(h3:text("Weapon")) select')
      .first();
    await expect(weaponSelect).toBeVisible();

    // Should have Colonial and Warden optgroups
    const optgroups = weaponSelect.locator("optgroup");
    const labels = await optgroups.evaluateAll((els) =>
      els.map((el) => el.getAttribute("label")),
    );
    expect(labels).toContain("Colonial");
    expect(labels).toContain("Warden");
  });

  test("wind input has 8 direction buttons and strength slider", async ({
    page,
  }) => {
    const windPanel = page.locator('.panel:has(h3:text("Wind"))');
    const windButtons = windPanel.locator(".wind-grid button");
    await expect(windButtons).toHaveCount(8);

    // Check direction labels
    const labels = await windButtons.allTextContents();
    expect(labels).toEqual(
      expect.arrayContaining(["NW", "N", "NE", "W", "E", "SW", "S", "SE"]),
    );

    // Strength slider
    const slider = windPanel.locator('input[type="range"]');
    await expect(slider).toBeVisible();
    await expect(slider).toHaveAttribute("min", "0");
    await expect(slider).toHaveAttribute("max", "5");
  });

  test("click on map places gun marker", async ({ page }) => {
    // Ensure Gun mode is active (default)
    const gunButton = page.locator(".placement-mode button", {
      hasText: "Gun",
    });
    await gunButton.click();

    // Click on the map
    const mapContainer = page.locator(".map-container");
    const box = await mapContainer.boundingBox();
    expect(box).not.toBeNull();

    // Click roughly in the center of the map
    await mapContainer.click({ position: { x: box!.width / 2, y: box!.height / 2 } });

    // SVG should now contain a gun marker (circle with green fill)
    const svg = page.locator(".map-container svg");
    const gunMarker = svg.locator('text:text("GUN")');
    await expect(gunMarker).toBeVisible({ timeout: 5000 });

    // Coordinate readout should show gun position
    const gunTag = page.locator(".coord-tag.gun-tag");
    await expect(gunTag).toBeVisible();
    const gunText = await gunTag.textContent();
    expect(gunText).toMatch(/GUN: [A-Q]\d+k\d/);
  });

  test("click on map places target marker", async ({ page }) => {
    // Switch to Target mode
    const targetButton = page.locator(".placement-mode button", {
      hasText: "Target",
    });
    await targetButton.click();

    const mapContainer = page.locator(".map-container");
    const box = await mapContainer.boundingBox();
    expect(box).not.toBeNull();

    // Click on the map
    await mapContainer.click({ position: { x: box!.width * 0.7, y: box!.height * 0.6 } });

    // SVG should now contain a target marker
    const svg = page.locator(".map-container svg");
    const targetLabel = svg.locator('text:text("TARGET")');
    await expect(targetLabel).toBeVisible({ timeout: 5000 });

    // Coordinate readout should show target position
    const targetTag = page.locator(".coord-tag.target-tag");
    await expect(targetTag).toBeVisible();
    const tgtText = await targetTag.textContent();
    expect(tgtText).toMatch(/TGT: [A-Q]\d+k\d/);
  });

  test("click on map places spotter marker", async ({ page }) => {
    // Switch to Spotter mode
    const spotterButton = page.locator(".placement-mode button", {
      hasText: "Spotter",
    });
    await spotterButton.click();

    const mapContainer = page.locator(".map-container");
    const box = await mapContainer.boundingBox();
    expect(box).not.toBeNull();

    await mapContainer.click({ position: { x: box!.width * 0.3, y: box!.height * 0.4 } });

    const svg = page.locator(".map-container svg");
    const spotterLabel = svg.locator('text:text("SPOTTER")');
    await expect(spotterLabel).toBeVisible({ timeout: 5000 });
  });

  test("placing gun and target shows firing solution prompt", async ({
    page,
  }) => {
    const mapContainer = page.locator(".map-container");
    const box = await mapContainer.boundingBox();
    expect(box).not.toBeNull();

    // Place gun
    await page
      .locator(".placement-mode button", { hasText: "Gun" })
      .click();
    await mapContainer.click({ position: { x: box!.width * 0.3, y: box!.height * 0.5 } });

    // Place target
    await page
      .locator(".placement-mode button", { hasText: "Target" })
      .click();
    await mapContainer.click({ position: { x: box!.width * 0.7, y: box!.height * 0.5 } });

    // Without a weapon selected, should prompt to select one
    const solutionPanel = page.locator(
      '.panel:has(h3:text("Firing Solution"))',
    );
    await expect(
      solutionPanel.locator("text=Select a weapon"),
    ).toBeVisible();
  });

  test("selecting weapon after placing gun+target shows firing solution", async ({
    page,
  }) => {
    const mapContainer = page.locator(".map-container");
    const box = await mapContainer.boundingBox();
    expect(box).not.toBeNull();

    // Place gun
    await page
      .locator(".placement-mode button", { hasText: "Gun" })
      .click();
    await mapContainer.click({ position: { x: box!.width * 0.3, y: box!.height * 0.5 } });

    // Place target
    await page
      .locator(".placement-mode button", { hasText: "Target" })
      .click();
    await mapContainer.click({ position: { x: box!.width * 0.7, y: box!.height * 0.3 } });

    // Select a weapon (pick the first actual weapon option)
    const weaponSelect = page
      .locator('.panel:has(h3:text("Weapon")) select')
      .first();
    const firstWeapon = weaponSelect.locator("optgroup option").first();
    const weaponValue = await firstWeapon.getAttribute("value");
    await weaponSelect.selectOption(weaponValue!);

    // Wait for the firing solution to calculate
    const solutionPanel = page.locator(
      '.panel:has(h3:text("Firing Solution"))',
    );
    await expect(solutionPanel.locator(".solution")).toBeVisible({
      timeout: 10_000,
    });

    // Should show azimuth and distance
    const azLabel = solutionPanel.locator('text="Azimuth"');
    await expect(azLabel).toBeVisible();
    const distLabel = solutionPanel.locator('text="Distance"');
    await expect(distLabel).toBeVisible();

    // Should show in-range or out-of-range status
    const statusLabel = solutionPanel.locator('text="Status"');
    await expect(statusLabel).toBeVisible();
  });

  test("weapon selection shows range circles on map", async ({ page }) => {
    const mapContainer = page.locator(".map-container");
    const box = await mapContainer.boundingBox();
    expect(box).not.toBeNull();

    // Place gun
    await page
      .locator(".placement-mode button", { hasText: "Gun" })
      .click();
    await mapContainer.click({ position: { x: box!.width / 2, y: box!.height / 2 } });

    // Select a weapon
    const weaponSelect = page
      .locator('.panel:has(h3:text("Weapon")) select')
      .first();
    const firstWeapon = weaponSelect.locator("optgroup option").first();
    const weaponValue = await firstWeapon.getAttribute("value");
    await weaponSelect.selectOption(weaponValue!);

    // Wait a moment for re-render
    await page.waitForTimeout(500);

    // SVG should now contain range circles (green max range, red min range)
    const svg = page.locator(".map-container svg");
    const circles = svg.locator("circle");
    const count = await circles.count();
    // Should have: gun marker circle + at least max range circle + min range circle = 3+
    expect(count).toBeGreaterThanOrEqual(3);
  });

  test("firing line renders between gun and target", async ({ page }) => {
    const mapContainer = page.locator(".map-container");
    const box = await mapContainer.boundingBox();
    expect(box).not.toBeNull();

    // Place gun
    await page
      .locator(".placement-mode button", { hasText: "Gun" })
      .click();
    await mapContainer.click({ position: { x: box!.width * 0.3, y: box!.height * 0.5 } });

    // Place target
    await page
      .locator(".placement-mode button", { hasText: "Target" })
      .click();
    await mapContainer.click({ position: { x: box!.width * 0.7, y: box!.height * 0.5 } });

    // SVG should contain a dashed line (firing line has stroke-dasharray)
    const svg = page.locator(".map-container svg");
    const dashedLines = svg.locator('line[stroke-dasharray="6 4"]');
    await expect(dashedLines).toHaveCount(1);
  });

  test("wind direction button toggles active state", async ({ page }) => {
    const windPanel = page.locator('.panel:has(h3:text("Wind"))');
    const northBtn = windPanel.locator(".wind-grid button", { hasText: "N" }).first();

    // Initially not active
    await expect(northBtn).not.toHaveClass(/active/);

    // Click to activate
    await northBtn.click();
    await expect(northBtn).toHaveClass(/active/);

    // Click again to deactivate
    await northBtn.click();
    await expect(northBtn).not.toHaveClass(/active/);
  });

  test("plan panel has name input and save button", async ({ page }) => {
    const planPanel = page.locator('.panel:has(h3:text("Plan"))');
    await expect(planPanel).toBeVisible();

    const nameInput = planPanel.locator('input[type="text"]');
    await expect(nameInput).toBeVisible();

    const saveButton = planPanel.locator("button", {
      hasText: "Save & Share",
    });
    await expect(saveButton).toBeVisible();
  });

  test("placement mode buttons highlight correctly", async ({ page }) => {
    const gunBtn = page.locator(".placement-mode button", { hasText: "Gun" });
    const targetBtn = page.locator(".placement-mode button", {
      hasText: "Target",
    });
    const spotterBtn = page.locator(".placement-mode button", {
      hasText: "Spotter",
    });

    // Gun should be active by default
    await expect(gunBtn).toHaveClass(/active-gun/);

    // Click Target
    await targetBtn.click();
    await expect(targetBtn).toHaveClass(/active-target/);
    await expect(gunBtn).not.toHaveClass(/active-gun/);

    // Click Spotter
    await spotterBtn.click();
    await expect(spotterBtn).toHaveClass(/active-spotter/);
    await expect(targetBtn).not.toHaveClass(/active-target/);
  });

  test("scroll wheel zooms in and increases transform scale", async ({
    page,
  }) => {
    const mapContainer = page.locator(".map-container");
    const mapInner = page.locator(".map-inner");
    await expect(mapContainer).toBeVisible();

    // Get initial transform
    const initialTransform = await mapInner.evaluate(
      (el) => getComputedStyle(el).transform,
    );

    // Scroll up (zoom in) on the map
    const box = await mapContainer.boundingBox();
    expect(box).not.toBeNull();
    await page.mouse.move(box!.x + box!.width / 2, box!.y + box!.height / 2);
    await page.mouse.wheel(0, -300);

    // Wait for the transform to update
    await page.waitForTimeout(200);

    const newTransform = await mapInner.evaluate(
      (el) => getComputedStyle(el).transform,
    );
    // Transform should have changed (zoomed in)
    expect(newTransform).not.toBe(initialTransform);

    // Extract scale from the matrix transform
    const scaleMatch = newTransform.match(/matrix\(([^,]+)/);
    if (scaleMatch) {
      const scale = parseFloat(scaleMatch[1]);
      expect(scale).toBeGreaterThan(1.0);
    }
  });

  test("scroll wheel zoom does not go below 1.0", async ({ page }) => {
    const mapContainer = page.locator(".map-container");
    const mapInner = page.locator(".map-inner");
    await expect(mapContainer).toBeVisible();

    const box = await mapContainer.boundingBox();
    expect(box).not.toBeNull();

    // Scroll down (zoom out) — should stay at 1.0
    await page.mouse.move(box!.x + box!.width / 2, box!.y + box!.height / 2);
    await page.mouse.wheel(0, 300);
    await page.waitForTimeout(200);

    const transform = await mapInner.evaluate(
      (el) => getComputedStyle(el).transform,
    );
    // Should still be identity (scale=1) or "none"
    const scaleMatch = transform.match(/matrix\(([^,]+)/);
    if (scaleMatch) {
      const scale = parseFloat(scaleMatch[1]);
      expect(scale).toBeCloseTo(1.0, 1);
    }
  });

  test("click-drag pans the map", async ({ page }) => {
    const mapContainer = page.locator(".map-container");
    const mapInner = page.locator(".map-inner");
    await expect(mapContainer).toBeVisible();

    const box = await mapContainer.boundingBox();
    expect(box).not.toBeNull();

    // First zoom in so there's room to pan
    await page.mouse.move(box!.x + box!.width / 2, box!.y + box!.height / 2);
    await page.mouse.wheel(0, -500);
    await page.waitForTimeout(200);

    // Get transform before drag
    const beforeDrag = await mapInner.evaluate(
      (el) => getComputedStyle(el).transform,
    );

    // Perform a click-drag
    const startX = box!.x + box!.width / 2;
    const startY = box!.y + box!.height / 2;
    await page.mouse.move(startX, startY);
    await page.mouse.down();
    await page.mouse.move(startX - 50, startY - 30, { steps: 5 });
    await page.mouse.up();
    await page.waitForTimeout(200);

    const afterDrag = await mapInner.evaluate(
      (el) => getComputedStyle(el).transform,
    );
    // Transform should have changed (panned)
    expect(afterDrag).not.toBe(beforeDrag);
  });

  test("click without drag still places markers correctly", async ({
    page,
  }) => {
    const mapContainer = page.locator(".map-container");
    await expect(mapContainer).toBeVisible();

    // Zoom in first
    const box = await mapContainer.boundingBox();
    expect(box).not.toBeNull();
    await page.mouse.move(box!.x + box!.width / 2, box!.y + box!.height / 2);
    await page.mouse.wheel(0, -300);
    await page.waitForTimeout(200);

    // Click (no drag) to place gun marker
    await page
      .locator(".placement-mode button", { hasText: "Gun" })
      .click();
    const updatedBox = await mapContainer.boundingBox();
    await page.mouse.click(
      updatedBox!.x + updatedBox!.width / 2,
      updatedBox!.y + updatedBox!.height / 2,
    );

    // Gun marker should appear
    const svg = page.locator(".map-container svg");
    const gunMarker = svg.locator('text:text("GUN")');
    await expect(gunMarker).toBeVisible({ timeout: 5000 });
  });

  test("double-click resets zoom to 1.0", async ({ page }) => {
    const mapContainer = page.locator(".map-container");
    const mapInner = page.locator(".map-inner");
    await expect(mapContainer).toBeVisible();

    const box = await mapContainer.boundingBox();
    expect(box).not.toBeNull();

    // Zoom in
    await page.mouse.move(box!.x + box!.width / 2, box!.y + box!.height / 2);
    await page.mouse.wheel(0, -500);
    await page.waitForTimeout(200);

    // Verify zoomed in
    const zoomedTransform = await mapInner.evaluate(
      (el) => getComputedStyle(el).transform,
    );
    const zoomMatch = zoomedTransform.match(/matrix\(([^,]+)/);
    if (zoomMatch) {
      expect(parseFloat(zoomMatch[1])).toBeGreaterThan(1.0);
    }

    // Double-click to reset
    await page.mouse.dblclick(
      box!.x + box!.width / 2,
      box!.y + box!.height / 2,
    );
    await page.waitForTimeout(200);

    // Should be back to scale=1
    const resetTransform = await mapInner.evaluate(
      (el) => getComputedStyle(el).transform,
    );
    const resetMatch = resetTransform.match(/matrix\(([^,]+)/);
    if (resetMatch) {
      expect(parseFloat(resetMatch[1])).toBeCloseTo(1.0, 1);
    }
  });

  test("changing map resets placed markers", async ({ page }) => {
    const mapContainer = page.locator(".map-container");
    const box = await mapContainer.boundingBox();
    expect(box).not.toBeNull();

    // Place gun
    await page
      .locator(".placement-mode button", { hasText: "Gun" })
      .click();
    await mapContainer.click({ position: { x: box!.width / 2, y: box!.height / 2 } });

    // Gun marker should be visible
    const svg = page.locator(".map-container svg");
    await expect(svg.locator('text:text("GUN")')).toBeVisible({ timeout: 5000 });

    // Change to a different map
    const mapSelect = page.locator(".sidebar .panel select").first();
    const secondOption = mapSelect.locator("option").nth(1);
    const secondValue = await secondOption.getAttribute("value");
    if (secondValue) {
      await mapSelect.selectOption(secondValue);

      // Gun marker should be gone (positions reset)
      await expect(svg.locator('text:text("GUN")')).not.toBeVisible({ timeout: 5000 });
    }
  });
});

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
    expect(texts).toContain("Active Weapon");
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

    // SVG should have correct viewBox matching map dimensions (2048x1776)
    const viewBox = await svgOverlay.getAttribute("viewBox");
    expect(viewBox).toBe("0 0 2048 1776");
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

  test("mousedown+mouseup places gun marker (no onclick)", async ({
    page,
  }) => {
    // This test exercises the exact event path used by our zoom/pan code:
    // onmousedown → onmouseup (no drag) → place marker.
    // The old onclick handler was removed in favour of this flow.
    await page
      .locator(".placement-mode button", { hasText: "Gun" })
      .click();

    const mapContainer = page.locator(".map-container");
    const box = await mapContainer.boundingBox();
    expect(box).not.toBeNull();

    const cx = box!.x + box!.width / 2;
    const cy = box!.y + box!.height / 2;

    // Explicit mousedown → mouseup with NO intervening mousemove
    await page.mouse.move(cx, cy);
    await page.mouse.down();
    await page.mouse.up();

    // Gun marker should appear
    const gunMarker = page.locator('.map-container svg text:text("GUN")');
    await expect(gunMarker).toBeVisible({ timeout: 5000 });

    // Coordinate readout should reflect the placement
    const gunTag = page.locator(".coord-tag.gun-tag");
    await expect(gunTag).toBeVisible();
    const text = await gunTag.textContent();
    expect(text).toMatch(/GUN: [A-Q]\d+k\d/);
  });

  test("marker y-coordinate matches click position (offset regression)", async ({
    page,
  }) => {
    // The image renders with width:100% + height:auto inside .map-inner.
    // The coordinate conversion must use the IMAGE dimensions (derived from
    // the aspect ratio), not the .map-container height (which is determined
    // by the CSS grid and may be taller or shorter than the image).
    await page
      .locator(".placement-mode button", { hasText: "Gun" })
      .click();

    // Use the image bounding box — it reflects the actual rendered map area
    const img = page.locator(".map-inner img");
    await expect(img).toBeVisible();
    const imgBox = await img.boundingBox();
    expect(imgBox).not.toBeNull();

    // Click at 25% down the image, horizontally centered.
    // 50% across 17 columns → column I (index 8)
    // 25% down  15 rows    → row 4    (index 3)
    const cx = imgBox!.x + imgBox!.width * 0.5;
    const cy = imgBox!.y + imgBox!.height * 0.25;
    await page.mouse.click(cx, cy);

    const gunTag = page.locator(".coord-tag.gun-tag");
    await expect(gunTag).toBeVisible();
    const text = await gunTag.textContent();
    expect(text).toContain("GUN: I4");
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

  test("placing gun and target without weapon shows no firing solution", async ({
    page,
  }) => {
    const mapContainer = page.locator(".map-container");
    const box = await mapContainer.boundingBox();
    expect(box).not.toBeNull();

    // Place gun (no weapon selected — default is empty)
    await page
      .locator(".placement-mode button", { hasText: "Gun" })
      .click();
    await mapContainer.click({ position: { x: box!.width * 0.3, y: box!.height * 0.5 } });

    // Place target (auto-pairs with the gun)
    await page
      .locator(".placement-mode button", { hasText: "Target" })
      .click();
    await mapContainer.click({ position: { x: box!.width * 0.7, y: box!.height * 0.5 } });

    // Gun and target coordinates should appear, but no solution (no weapon)
    const solutionPanel = page.locator(
      '.panel:has(h3:text("Firing Solution"))',
    );
    await expect(solutionPanel.locator(".coord-info.gun-coord")).toBeVisible({ timeout: 5000 });
    await expect(solutionPanel.locator(".coord-info.target-coord")).toBeVisible();
    await expect(solutionPanel.locator(".solution")).not.toBeVisible();
  });

  test("weapon + gun + target placement shows firing solution", async ({
    page,
  }) => {
    const mapContainer = page.locator(".map-container");
    const box = await mapContainer.boundingBox();
    expect(box).not.toBeNull();

    // Select a weapon FIRST so the gun inherits it on placement
    const weaponSelect = page
      .locator('.panel:has(h3:text("Active Weapon")) select')
      .first();
    const firstWeapon = weaponSelect.locator("optgroup option").first();
    const weaponValue = await firstWeapon.getAttribute("value");
    await weaponSelect.selectOption(weaponValue!);

    // Place gun (inherits selected weapon)
    await page
      .locator(".placement-mode button", { hasText: "Gun" })
      .click();
    await mapContainer.click({ position: { x: box!.width * 0.3, y: box!.height * 0.5 } });

    // Place target (auto-pairs with the gun)
    await page
      .locator(".placement-mode button", { hasText: "Target" })
      .click();
    await mapContainer.click({ position: { x: box!.width * 0.7, y: box!.height * 0.3 } });

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

    // Select a weapon FIRST so the gun inherits it
    const weaponSelect = page
      .locator('.panel:has(h3:text("Active Weapon")) select')
      .first();
    const firstWeapon = weaponSelect.locator("optgroup option").first();
    const weaponValue = await firstWeapon.getAttribute("value");
    await weaponSelect.selectOption(weaponValue!);

    // Place gun (inherits selected weapon)
    await page
      .locator(".placement-mode button", { hasText: "Gun" })
      .click();
    await mapContainer.click({ position: { x: box!.width / 2, y: box!.height / 2 } });

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
    const dashedLines = svg.locator('line[stroke-dasharray="12 8"]');
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

  test("multiple gun clicks place multiple markers", async ({ page }) => {
    const gunBtn = page.locator(".placement-mode button", { hasText: "Gun" });
    const mapContainer = page.locator(".map-container");
    const box = await mapContainer.boundingBox();
    expect(box).not.toBeNull();

    // Place 3 guns — re-select Gun mode each time because auto-cycle switches to Target
    await gunBtn.click();
    await mapContainer.click({ position: { x: box!.width * 0.2, y: box!.height * 0.3 } });
    await gunBtn.click();
    await mapContainer.click({ position: { x: box!.width * 0.4, y: box!.height * 0.5 } });
    await gunBtn.click();
    await mapContainer.click({ position: { x: box!.width * 0.6, y: box!.height * 0.7 } });

    // Should see 3 gun markers (labeled GUN 1, GUN 2, GUN 3)
    const svg = page.locator(".map-container svg");
    await expect(svg.locator('text:text("GUN 1")')).toBeVisible({ timeout: 5000 });
    await expect(svg.locator('text:text("GUN 2")')).toBeVisible();
    await expect(svg.locator('text:text("GUN 3")')).toBeVisible();

    // Should see 3 gun coord tags
    const gunTags = page.locator(".coord-tag.gun-tag");
    await expect(gunTags).toHaveCount(3);
  });

  test("right-click removes nearest marker", async ({ page }) => {
    await page
      .locator(".placement-mode button", { hasText: "Gun" })
      .click();

    const mapContainer = page.locator(".map-container");
    const box = await mapContainer.boundingBox();
    expect(box).not.toBeNull();

    // Place a gun marker
    const cx = box!.width * 0.5;
    const cy = box!.height * 0.5;
    await mapContainer.click({ position: { x: cx, y: cy } });

    const svg = page.locator(".map-container svg");
    await expect(svg.locator('text:text("GUN")')).toBeVisible({ timeout: 5000 });

    // Right-click near the same spot to remove it
    await mapContainer.click({
      position: { x: cx, y: cy },
      button: "right",
    });

    // Gun marker should be gone
    await expect(svg.locator('text:text("GUN")')).not.toBeVisible({ timeout: 5000 });
  });

  test("right-click removes correct marker from multiple", async ({ page }) => {
    const gunBtn = page.locator(".placement-mode button", { hasText: "Gun" });
    const mapContainer = page.locator(".map-container");
    const box = await mapContainer.boundingBox();
    expect(box).not.toBeNull();

    // Place 2 guns — re-select Gun mode after auto-cycle
    await gunBtn.click();
    await mapContainer.click({ position: { x: box!.width * 0.3, y: box!.height * 0.5 } });
    await gunBtn.click();
    await mapContainer.click({ position: { x: box!.width * 0.7, y: box!.height * 0.5 } });

    const svg = page.locator(".map-container svg");
    await expect(svg.locator('text:text("GUN 1")')).toBeVisible({ timeout: 5000 });
    await expect(svg.locator('text:text("GUN 2")')).toBeVisible();

    // Switch back to Gun mode so right-click checks guns first
    await gunBtn.click();

    // Right-click near the first gun to remove it
    await mapContainer.click({
      position: { x: box!.width * 0.3, y: box!.height * 0.5 },
      button: "right",
    });

    // After removal, only one gun remains — label goes back to "GUN" (no number)
    await expect(svg.locator('text:text("GUN")')).toBeVisible({ timeout: 5000 });
    await expect(svg.locator('text:text("GUN 1")')).not.toBeVisible();
    await expect(svg.locator('text:text("GUN 2")')).not.toBeVisible();
  });

  test("map bottom is reachable by panning at zoom 1", async ({ page }) => {
    // Regression: clamp_pan used to assume content height == container height,
    // preventing downward panning when the map image (width:100%, height:auto)
    // was taller than the map-container.
    const mapContainer = page.locator(".map-container");
    const mapInner = page.locator(".map-inner");
    const img = page.locator(".map-inner img");
    await expect(img).toBeVisible();

    const containerBox = await mapContainer.boundingBox();
    const imgBox = await img.boundingBox();
    expect(containerBox).not.toBeNull();
    expect(imgBox).not.toBeNull();

    // Only relevant when the image is taller than the container
    if (imgBox!.height <= containerBox!.height) {
      // Map fits — nothing to test; skip gracefully
      return;
    }

    // The bottom portion of the image is clipped. Try to pan down.
    const startX = containerBox!.x + containerBox!.width / 2;
    const startY = containerBox!.y + containerBox!.height / 2;
    await page.mouse.move(startX, startY);
    await page.mouse.down();
    // Drag upward to pan the map down (reveal bottom)
    await page.mouse.move(startX, startY - 200, { steps: 10 });
    await page.mouse.up();
    await page.waitForTimeout(200);

    // Extract translateY from the CSS transform matrix
    const transform = await mapInner.evaluate(
      (el) => getComputedStyle(el).transform,
    );
    // matrix(a, b, c, d, tx, ty) — ty is the 6th value
    const matrixMatch = transform.match(
      /matrix\(([^,]+),\s*([^,]+),\s*([^,]+),\s*([^,]+),\s*([^,]+),\s*([^,]+)\)/,
    );
    expect(matrixMatch).not.toBeNull();
    const ty = parseFloat(matrixMatch![6]);
    // Pan should have moved negative (map scrolled down)
    expect(ty).toBeLessThan(-10);
  });

  test("gun placed at bottom of map after panning down", async ({
    page,
  }) => {
    // Pan the map down to reveal the bottom, then place a gun there.
    // This verifies that the panning fix actually lets users interact
    // with the bottom portion of the map.
    const mapContainer = page.locator(".map-container");
    const img = page.locator(".map-inner img");
    await expect(img).toBeVisible();

    const containerBox = await mapContainer.boundingBox();
    const imgBox = await img.boundingBox();
    expect(containerBox).not.toBeNull();
    expect(imgBox).not.toBeNull();

    // Only relevant when the image is taller than the container
    if (imgBox!.height <= containerBox!.height) {
      return;
    }

    // Pan down by dragging upward
    const startX = containerBox!.x + containerBox!.width / 2;
    const startY = containerBox!.y + containerBox!.height / 2;
    const panAmount = imgBox!.height - containerBox!.height;
    await page.mouse.move(startX, startY);
    await page.mouse.down();
    await page.mouse.move(startX, startY - panAmount, { steps: 10 });
    await page.mouse.up();
    await page.waitForTimeout(200);

    // Now place a gun near the bottom of the visible area
    await page
      .locator(".placement-mode button", { hasText: "Gun" })
      .click();

    // Click near the bottom of the container (now showing the map bottom)
    const clickX = containerBox!.x + containerBox!.width / 2;
    const clickY = containerBox!.y + containerBox!.height * 0.9;
    await page.mouse.click(clickX, clickY);

    // Gun marker should appear
    const svg = page.locator(".map-container svg");
    await expect(svg.locator('text:text("GUN")')).toBeVisible({ timeout: 5000 });

    // The coordinate should be in the bottom rows (row 12-15)
    const gunTag = page.locator(".coord-tag.gun-tag");
    await expect(gunTag).toBeVisible();
    const text = await gunTag.textContent();
    expect(text).toMatch(/GUN: [A-Q]1[2-5]k\d/);
  });

  test("keyboard shortcuts work on fresh page load without clicking", async ({ page }) => {
    // Don't click anything — just press 't' directly after page load
    await page.keyboard.press("t");

    // Target button should be active
    await expect(page.locator(".placement-mode button", { hasText: "Target" })).toHaveClass(/active-target/, { timeout: 2000 });
  });

  test("keyboard shortcut 'g' switches to gun mode after clicking map", async ({ page }) => {
    // Switch to spotter mode (which doesn't auto-cycle on placement)
    await page.locator(".placement-mode button", { hasText: "Spotter" }).click();
    await expect(page.locator(".placement-mode button", { hasText: "Spotter" })).toHaveClass(/active-spotter/);

    // Click on the map — simulates normal user workflow of placing a marker
    const mapContainer = page.locator(".map-container");
    const box = await mapContainer.boundingBox();
    expect(box).not.toBeNull();
    await mapContainer.click({ position: { x: box!.width / 2, y: box!.height / 2 } });

    // Press 'g' to switch to gun mode
    await page.keyboard.press("g");

    // Gun button should be active
    await expect(page.locator(".placement-mode button", { hasText: "Gun" })).toHaveClass(/active-gun/, { timeout: 2000 });
  });

  test("keyboard shortcut 't' switches to target mode after clicking map", async ({ page }) => {
    // Start in spotter mode
    await page.locator(".placement-mode button", { hasText: "Spotter" }).click();

    // Click on the map
    const mapContainer = page.locator(".map-container");
    const box = await mapContainer.boundingBox();
    expect(box).not.toBeNull();
    await mapContainer.click({ position: { x: box!.width / 2, y: box!.height / 2 } });

    // Press 't' to switch to target mode
    await page.keyboard.press("t");

    await expect(page.locator(".placement-mode button", { hasText: "Target" })).toHaveClass(/active-target/, { timeout: 2000 });
  });

  test("keyboard shortcut 's' switches to spotter mode after clicking map", async ({ page }) => {
    // Default mode is Gun; clicking map places a gun and auto-cycles to Target
    const mapContainer = page.locator(".map-container");
    const box = await mapContainer.boundingBox();
    expect(box).not.toBeNull();
    await mapContainer.click({ position: { x: box!.width / 2, y: box!.height / 2 } });

    // After placement, mode is Target (auto-cycled from Gun)
    await expect(page.locator(".placement-mode button", { hasText: "Target" })).toHaveClass(/active-target/);

    // Press 's' to switch to spotter mode
    await page.keyboard.press("s");

    await expect(page.locator(".placement-mode button", { hasText: "Spotter" })).toHaveClass(/active-spotter/, { timeout: 2000 });
  });

  test("keyboard shortcut 'r' resets zoom after clicking map", async ({ page }) => {
    const mapContainer = page.locator(".map-container");
    const mapInner = page.locator(".map-inner");
    const box = await mapContainer.boundingBox();
    expect(box).not.toBeNull();

    // Zoom in with scroll wheel
    await page.mouse.move(box!.x + box!.width / 2, box!.y + box!.height / 2);
    await page.mouse.wheel(0, -300);
    await page.waitForTimeout(200);

    // Verify zoomed in (scale > 1)
    const zoomedTransform = await mapInner.evaluate(
      (el) => getComputedStyle(el).transform,
    );
    expect(zoomedTransform).not.toBe("none");
    // Parse matrix(a, ...) — a is the X scale
    const zoomedScale = parseFloat(zoomedTransform.split("(")[1]);
    expect(zoomedScale).toBeGreaterThan(1);

    // Click on the map to simulate user interaction (loses focus to non-focusable area)
    await mapContainer.click({ position: { x: box!.width / 2, y: box!.height / 2 } });

    // Press 'r' to reset zoom
    await page.keyboard.press("r");
    await page.waitForTimeout(200);

    // Verify zoom is back to 1.0
    const resetTransform = await mapInner.evaluate(
      (el) => getComputedStyle(el).transform,
    );
    const resetScale = parseFloat(resetTransform.split("(")[1]);
    expect(resetScale).toBeCloseTo(1.0, 1);
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

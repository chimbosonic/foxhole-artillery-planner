import { test, expect } from "@playwright/test";

test.describe("Foxhole Artillery Planner", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/");
    // Wait for WASM app to hydrate - the app div with class "app" should be present
    await page.waitForSelector(".app", { timeout: 15_000 });
  });

  test("page loads with Warden theme and correct title", async ({ page }) => {
    await expect(page).toHaveTitle("Foxhole Artillery Planner");

    // Verify Warden theme is applied (CSS loaded)
    const body = page.locator("body");
    const bgColor = await body.evaluate(
      (el) => getComputedStyle(el).backgroundColor,
    );
    // --bg-dark: #141c28 → rgb(20, 28, 40)
    expect(bgColor).toBe("rgb(20, 28, 40)");
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

test.describe("Placement tracking", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/");
    await page.waitForSelector(".app", { timeout: 15_000 });
  });

  test("placing a gun increments placement stats", async ({ page }) => {
    // Select a weapon first so the gun has a slug to track
    const weaponSelect = page
      .locator('.panel:has(h3:text("Active Weapon")) select')
      .first();
    const firstWeapon = weaponSelect.locator("optgroup option").first();
    const weaponSlug = await firstWeapon.getAttribute("value");
    expect(weaponSlug).toBeTruthy();
    await weaponSelect.selectOption(weaponSlug!);

    // Place a gun on the map
    await page.locator(".placement-mode button", { hasText: "Gun" }).click();
    const mapContainer = page.locator(".map-container");
    const box = await mapContainer.boundingBox();
    expect(box).not.toBeNull();
    await mapContainer.click({ position: { x: box!.width / 2, y: box!.height / 2 } });

    // Wait for the fire-and-forget tracking call to complete
    await page.waitForTimeout(1000);

    // Query stats via GraphQL
    const resp = await page.request.post("/graphql", {
      data: {
        query: `{ stats { gunPlacements { weaponSlug count } } }`,
      },
    });
    const json = await resp.json();
    const placements = json.data.stats.gunPlacements;
    const entry = placements.find((p: any) => p.weaponSlug === weaponSlug);
    expect(entry).toBeTruthy();
    expect(entry.count).toBeGreaterThanOrEqual(1);
  });

  test("stats show per-faction totals", async ({ page }) => {
    // Find a Colonial weapon and a Warden weapon
    const weaponSelect = page
      .locator('.panel:has(h3:text("Active Weapon")) select')
      .first();
    const colonialWeapon = weaponSelect.locator('optgroup[label="Colonial"] option').first();
    const wardenWeapon = weaponSelect.locator('optgroup[label="Warden"] option').first();

    const colonialSlug = await colonialWeapon.getAttribute("value");
    const wardenSlug = await wardenWeapon.getAttribute("value");
    expect(colonialSlug).toBeTruthy();
    expect(wardenSlug).toBeTruthy();

    const mapContainer = page.locator(".map-container");
    const box = await mapContainer.boundingBox();
    expect(box).not.toBeNull();

    // Place a Colonial gun
    await weaponSelect.selectOption(colonialSlug!);
    await page.locator(".placement-mode button", { hasText: "Gun" }).click();
    await mapContainer.click({ position: { x: box!.width * 0.3, y: box!.height * 0.5 } });

    // Place a Warden gun (re-select Gun mode since auto-cycle switches to Target)
    await weaponSelect.selectOption(wardenSlug!);
    await page.locator(".placement-mode button", { hasText: "Gun" }).click();
    await mapContainer.click({ position: { x: box!.width * 0.6, y: box!.height * 0.5 } });

    await page.waitForTimeout(1000);

    const resp = await page.request.post("/graphql", {
      data: {
        query: `{ stats { gunPlacementTotals { colonial warden total } } }`,
      },
    });
    const json = await resp.json();
    const totals = json.data.stats.gunPlacementTotals;
    expect(totals.colonial).toBeGreaterThanOrEqual(1);
    expect(totals.warden).toBeGreaterThanOrEqual(1);
    expect(totals.total).toBeGreaterThanOrEqual(2);
  });

  test("stats include weapon display name and faction", async ({ page }) => {
    const weaponSelect = page
      .locator('.panel:has(h3:text("Active Weapon")) select')
      .first();
    const firstWeapon = weaponSelect.locator("optgroup option").first();
    const weaponSlug = await firstWeapon.getAttribute("value");
    expect(weaponSlug).toBeTruthy();
    await weaponSelect.selectOption(weaponSlug!);

    // Place a gun
    await page.locator(".placement-mode button", { hasText: "Gun" }).click();
    const mapContainer = page.locator(".map-container");
    const box = await mapContainer.boundingBox();
    expect(box).not.toBeNull();
    await mapContainer.click({ position: { x: box!.width / 2, y: box!.height / 2 } });

    await page.waitForTimeout(1000);

    const resp = await page.request.post("/graphql", {
      data: {
        query: `{ stats { gunPlacements { weaponSlug displayName faction count } } }`,
      },
    });
    const json = await resp.json();
    const placements = json.data.stats.gunPlacements;
    const entry = placements.find((p: any) => p.weaponSlug === weaponSlug);
    expect(entry).toBeTruthy();
    expect(entry.displayName).toBeTruthy();
    expect(entry.displayName.length).toBeGreaterThan(0);
    expect(["COLONIAL", "WARDEN", "BOTH"]).toContain(entry.faction);
    expect(entry.count).toBeGreaterThanOrEqual(1);
  });

  test("placing a gun without a weapon tracks as unassigned", async ({ page }) => {
    // Ensure no weapon is selected (default empty)
    const weaponSelect = page
      .locator('.panel:has(h3:text("Active Weapon")) select')
      .first();
    await weaponSelect.selectOption("");

    // Place a gun on the map
    await page.locator(".placement-mode button", { hasText: "Gun" }).click();
    const mapContainer = page.locator(".map-container");
    const box = await mapContainer.boundingBox();
    expect(box).not.toBeNull();
    await mapContainer.click({ position: { x: box!.width / 2, y: box!.height / 2 } });

    await page.waitForTimeout(1000);

    // Query stats and verify "unassigned" entry exists
    const resp = await page.request.post("/graphql", {
      data: {
        query: `{ stats { gunPlacements { weaponSlug displayName faction count } } }`,
      },
    });
    const json = await resp.json();
    const placements = json.data.stats.gunPlacements;
    const entry = placements.find((p: any) => p.weaponSlug === "unassigned");
    expect(entry).toBeTruthy();
    expect(entry.displayName).toBe("Unassigned");
    expect(entry.faction).toBe("BOTH");
    expect(entry.count).toBeGreaterThanOrEqual(1);
  });

  test("placing a target increments target stats", async ({ page }) => {
    // Switch to Target mode
    await page.locator(".placement-mode button", { hasText: "Target" }).click();

    // Place a target on the map
    const mapContainer = page.locator(".map-container");
    const box = await mapContainer.boundingBox();
    expect(box).not.toBeNull();
    await mapContainer.click({ position: { x: box!.width / 2, y: box!.height / 2 } });

    // Wait for the fire-and-forget tracking call to complete
    await page.waitForTimeout(1000);

    // Query stats via GraphQL
    const resp = await page.request.post("/graphql", {
      data: {
        query: `{ stats { markerPlacements { targets } } }`,
      },
    });
    const json = await resp.json();
    expect(json.data.stats.markerPlacements.targets).toBeGreaterThanOrEqual(1);
  });

  test("placing a spotter increments spotter stats", async ({ page }) => {
    // Switch to Spotter mode
    await page.locator(".placement-mode button", { hasText: "Spotter" }).click();

    // Place a spotter on the map
    const mapContainer = page.locator(".map-container");
    const box = await mapContainer.boundingBox();
    expect(box).not.toBeNull();
    await mapContainer.click({ position: { x: box!.width / 2, y: box!.height / 2 } });

    // Wait for the fire-and-forget tracking call to complete
    await page.waitForTimeout(1000);

    // Query stats via GraphQL
    const resp = await page.request.post("/graphql", {
      data: {
        query: `{ stats { markerPlacements { spotters } } }`,
      },
    });
    const json = await resp.json();
    expect(json.data.stats.markerPlacements.spotters).toBeGreaterThanOrEqual(1);
  });
});

test.describe("Warden theme", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/");
    await page.waitForSelector(".app", { timeout: 15_000 });
  });

  test("CSS custom properties use Warden color palette", async ({ page }) => {
    const root = page.locator(":root");
    const vars = await root.evaluate((el) => {
      const s = getComputedStyle(el);
      return {
        bgDark: s.getPropertyValue("--bg-dark").trim(),
        bgPanel: s.getPropertyValue("--bg-panel").trim(),
        bgInput: s.getPropertyValue("--bg-input").trim(),
        accent: s.getPropertyValue("--accent").trim(),
        accentGreen: s.getPropertyValue("--accent-green").trim(),
        accentBlue: s.getPropertyValue("--accent-blue").trim(),
        text: s.getPropertyValue("--text").trim(),
        textDim: s.getPropertyValue("--text-dim").trim(),
        border: s.getPropertyValue("--border").trim(),
      };
    });

    expect(vars.bgDark).toBe("#141c28");
    expect(vars.bgPanel).toBe("#1a2840");
    expect(vars.bgInput).toBe("#1f3358");
    expect(vars.accent).toBe("#cf8e3e");
    expect(vars.accentGreen).toBe("#5ab882");
    expect(vars.accentBlue).toBe("#4a8fd4");
    expect(vars.text).toBe("#d6dce6");
    expect(vars.textDim).toBe("#7888a0");
    expect(vars.border).toBe("#283a58");
  });

  test("header uses Warden accent color", async ({ page }) => {
    const h1 = page.locator(".header h1");
    const color = await h1.evaluate((el) => getComputedStyle(el).color);
    // --accent: #cf8e3e → rgb(207, 142, 62)
    expect(color).toBe("rgb(207, 142, 62)");
  });

  test("sidebar panel uses Warden navy background", async ({ page }) => {
    const sidebar = page.locator(".sidebar");
    const bgColor = await sidebar.evaluate(
      (el) => getComputedStyle(el).backgroundColor,
    );
    // --bg-panel: #1a2840 → rgb(26, 40, 64)
    expect(bgColor).toBe("rgb(26, 40, 64)");
  });

  test("form inputs use Warden blue background", async ({ page }) => {
    const select = page.locator(".sidebar .panel select").first();
    const bgColor = await select.evaluate(
      (el) => getComputedStyle(el).backgroundColor,
    );
    // --bg-input: #1f3358 → rgb(31, 51, 88)
    expect(bgColor).toBe("rgb(31, 51, 88)");
  });

  test("gun marker uses tactical green", async ({ page }) => {
    await page
      .locator(".placement-mode button", { hasText: "Gun" })
      .click();

    const mapContainer = page.locator(".map-container");
    const box = await mapContainer.boundingBox();
    expect(box).not.toBeNull();
    await mapContainer.click({
      position: { x: box!.width / 2, y: box!.height / 2 },
    });

    // Gun markers are inline SVG circles with fill="#5ab882"
    const gunMarker = page.locator('.map-container svg circle[fill="#5ab882"]');
    await expect(gunMarker.first()).toBeVisible({ timeout: 5000 });

    const fill = await gunMarker.first().getAttribute("fill");
    expect(fill).toBe("#5ab882");
  });

  test("target marker uses amber accent", async ({ page }) => {
    await page
      .locator(".placement-mode button", { hasText: "Target" })
      .click();

    const mapContainer = page.locator(".map-container");
    const box = await mapContainer.boundingBox();
    expect(box).not.toBeNull();
    await mapContainer.click({
      position: { x: box!.width / 2, y: box!.height / 2 },
    });

    // Target markers are inline SVG circles with fill="#cf8e3e"
    const targetMarker = page.locator('.map-container svg circle[fill="#cf8e3e"]');
    await expect(targetMarker.first()).toBeVisible({ timeout: 5000 });

    const fill = await targetMarker.first().getAttribute("fill");
    expect(fill).toBe("#cf8e3e");
  });
});

test.describe("Theme toggle", () => {
  test.beforeEach(async ({ page }) => {
    // Clear localStorage to start fresh each test
    await page.goto("/");
    await page.evaluate(() => localStorage.removeItem("faction"));
    await page.reload();
    await page.waitForSelector(".app", { timeout: 15_000 });
  });

  test("faction toggle renders in header with Warden active", async ({ page }) => {
    const toggle = page.locator(".header .faction-toggle");
    await expect(toggle).toBeVisible();

    const wardenBtn = toggle.locator("button", { hasText: "Warden" });
    const colonialBtn = toggle.locator("button", { hasText: "Colonial" });
    await expect(wardenBtn).toBeVisible();
    await expect(colonialBtn).toBeVisible();
    await expect(wardenBtn).toHaveClass(/active/);
    await expect(colonialBtn).not.toHaveClass(/active/);
  });

  test("clicking Colonial switches to Colonial theme", async ({ page }) => {
    const colonialBtn = page.locator(".header .faction-toggle button", { hasText: "Colonial" });
    await colonialBtn.click();

    // Colonial button should be active, Warden should not
    await expect(colonialBtn).toHaveClass(/active/);
    const wardenBtn = page.locator(".header .faction-toggle button", { hasText: "Warden" });
    await expect(wardenBtn).not.toHaveClass(/active/);

    // App div should have colonial class
    const app = page.locator(".app");
    await expect(app).toHaveClass(/colonial/);

    // CSS custom properties should reflect Colonial palette
    const accent = await app.evaluate((el) =>
      getComputedStyle(el).getPropertyValue("--accent").trim(),
    );
    expect(accent).toBe("#6fbf5e");
  });

  test("clicking Warden switches back to Warden theme", async ({ page }) => {
    // Switch to Colonial first
    const colonialBtn = page.locator(".header .faction-toggle button", { hasText: "Colonial" });
    await colonialBtn.click();
    await expect(colonialBtn).toHaveClass(/active/);

    // Switch back to Warden
    const wardenBtn = page.locator(".header .faction-toggle button", { hasText: "Warden" });
    await wardenBtn.click();
    await expect(wardenBtn).toHaveClass(/active/);
    await expect(colonialBtn).not.toHaveClass(/active/);

    // App div should NOT have colonial class
    const app = page.locator(".app");
    await expect(app).not.toHaveClass(/colonial/);

    // CSS should be back to Warden
    const accent = await app.evaluate((el) =>
      getComputedStyle(el).getPropertyValue("--accent").trim(),
    );
    expect(accent).toBe("#cf8e3e");
  });

  test("localStorage faction persists across reload", async ({ page }) => {
    // Click Colonial
    const colonialBtn = page.locator(".header .faction-toggle button", { hasText: "Colonial" });
    await colonialBtn.click();

    // Verify Colonial is active
    const app = page.locator(".app");
    await expect(app).toHaveClass(/colonial/);
    await expect(colonialBtn).toHaveClass(/active/);

    // Reload the page
    await page.reload();
    await page.waitForSelector(".app", { timeout: 15_000 });

    // Verify Colonial persisted
    await expect(page.locator(".app")).toHaveClass(/colonial/);
    await expect(
      page.locator(".header .faction-toggle button", { hasText: "Colonial" }),
    ).toHaveClass(/active/);
  });

  test("gun and target markers render correctly in Colonial theme", async ({
    page,
  }) => {
    // Switch to Colonial
    const colonialBtn = page.locator(".header .faction-toggle button", { hasText: "Colonial" });
    await colonialBtn.click();
    await expect(colonialBtn).toHaveClass(/active/);

    // Place a gun
    await page
      .locator(".placement-mode button", { hasText: "Gun" })
      .click();
    const mapContainer = page.locator(".map-container");
    const box = await mapContainer.boundingBox();
    expect(box).not.toBeNull();
    await mapContainer.click({
      position: { x: box!.width * 0.3, y: box!.height / 2 },
    });

    // Place a target
    await page
      .locator(".placement-mode button", { hasText: "Target" })
      .click();
    await mapContainer.click({
      position: { x: box!.width * 0.7, y: box!.height / 2 },
    });

    // Gun marker should still use green (#5ab882)
    const gunMarker = page.locator(
      '.map-container svg circle[fill="#5ab882"]',
    );
    await expect(gunMarker.first()).toBeVisible({ timeout: 5000 });

    // Target marker should use Colonial green (#6fbf5e)
    const targetMarker = page.locator(
      '.map-container svg circle[fill="#6fbf5e"]',
    );
    await expect(targetMarker.first()).toBeVisible({ timeout: 5000 });
  });
});

test.describe("Plan save and load", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/");
    await page.waitForSelector(".app", { timeout: 15_000 });
  });

  test("saving a plan and loading it preserves markers", async ({ page }) => {
    const mapContainer = page.locator(".map-container");
    const box = await mapContainer.boundingBox();
    expect(box).not.toBeNull();

    // Select a weapon so we have a full plan
    const weaponSelect = page
      .locator('.panel:has(h3:text("Active Weapon")) select')
      .first();
    const firstWeapon = weaponSelect.locator("optgroup option").first();
    const weaponValue = await firstWeapon.getAttribute("value");
    await weaponSelect.selectOption(weaponValue!);

    // Place a gun
    await page
      .locator(".placement-mode button", { hasText: "Gun" })
      .click();
    await mapContainer.click({
      position: { x: box!.width * 0.3, y: box!.height * 0.4 },
    });

    // Place a target
    await page
      .locator(".placement-mode button", { hasText: "Target" })
      .click();
    await mapContainer.click({
      position: { x: box!.width * 0.7, y: box!.height * 0.6 },
    });

    // Wait for markers to appear
    const svg = page.locator(".map-container svg");
    await expect(svg.locator('text:text("GUN")')).toBeVisible({ timeout: 5000 });
    await expect(svg.locator('text:text("TARGET")')).toBeVisible({ timeout: 5000 });

    // Type a custom plan name
    const planPanel = page.locator('.panel:has(h3:text("Plan"))');
    const nameInput = planPanel.locator('input[type="text"]');
    await nameInput.fill("E2E Test Plan");

    // Click Save & Share
    const saveButton = planPanel.locator("button", { hasText: "Save & Share" });
    await saveButton.click();

    // Wait for the plan URL to appear
    const planUrlInput = planPanel.locator(".plan-url input[readonly]");
    await expect(planUrlInput).toBeVisible({ timeout: 10_000 });

    // Extract the plan URL path
    const planUrl = await planUrlInput.inputValue();
    expect(planUrl).toContain("/plan/");
    const urlPath = new URL(planUrl).pathname;

    // Navigate to the saved plan URL
    await page.goto(urlPath);
    await page.waitForSelector(".app", { timeout: 15_000 });

    // Verify gun and target markers are present in SVG
    const loadedSvg = page.locator(".map-container svg");
    await expect(loadedSvg.locator('text:text("GUN")')).toBeVisible({ timeout: 10_000 });
    await expect(loadedSvg.locator('text:text("TARGET")')).toBeVisible({ timeout: 10_000 });

    // Verify coordinate readout tags show positions
    await expect(page.locator(".coord-tag.gun-tag")).toBeVisible();
    await expect(page.locator(".coord-tag.target-tag")).toBeVisible();
  });
});

test.describe("Error handling", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/");
    await page.waitForSelector(".app", { timeout: 15_000 });
  });

  test("shows alert on save failure", async ({ page }) => {
    // Intercept GraphQL endpoint and return 500 for mutations
    await page.route("**/graphql", async (route) => {
      const request = route.request();
      const postData = request.postData();
      if (postData && postData.includes("createPlan")) {
        await route.fulfill({
          status: 500,
          contentType: "application/json",
          body: JSON.stringify({ errors: [{ message: "Server error" }] }),
        });
      } else {
        await route.continue();
      }
    });

    // Place a gun on the map
    const mapContainer = page.locator(".map-container");
    const box = await mapContainer.boundingBox();
    expect(box).not.toBeNull();
    await page
      .locator(".placement-mode button", { hasText: "Gun" })
      .click();
    await mapContainer.click({
      position: { x: box!.width / 2, y: box!.height / 2 },
    });

    // Set up dialog handler before clicking save
    const dialogPromise = page.waitForEvent("dialog");

    // Click Save & Share
    const planPanel = page.locator('.panel:has(h3:text("Plan"))');
    const saveButton = planPanel.locator("button", { hasText: "Save & Share" });
    await saveButton.click();

    // Verify alert dialog appears
    const dialog = await dialogPromise;
    expect(dialog.type()).toBe("alert");
    expect(dialog.message().length).toBeGreaterThan(0);
    await dialog.accept();
  });
});

test.describe("Responsive layout", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/");
    await page.waitForSelector(".app", { timeout: 15_000 });
  });

  test.describe("Desktop viewport (1280x720)", () => {
    test.beforeEach(async ({ page }) => {
      await page.setViewportSize({ width: 1280, height: 720 });
    });

    test("sidebar is visible by default", async ({ page }) => {
      const sidebar = page.locator(".sidebar");
      await expect(sidebar).toBeVisible();
    });

    test("hamburger button is hidden", async ({ page }) => {
      const toggle = page.locator(".sidebar-toggle");
      await expect(toggle).toBeHidden();
    });

    test("sidebar backdrop is not visible", async ({ page }) => {
      const backdrop = page.locator(".sidebar-backdrop");
      await expect(backdrop).toBeHidden();
    });

    test("map container is visible alongside sidebar", async ({ page }) => {
      const map = page.locator(".map-container");
      await expect(map).toBeVisible();
      const sidebar = page.locator(".sidebar");
      await expect(sidebar).toBeVisible();
    });
  });

  test.describe("Mobile viewport (375x667)", () => {
    test.beforeEach(async ({ page }) => {
      await page.setViewportSize({ width: 375, height: 667 });
    });

    test("sidebar is hidden by default", async ({ page }) => {
      const sidebar = page.locator(".sidebar");
      await expect(sidebar).not.toBeInViewport();
    });

    test("hamburger button is visible", async ({ page }) => {
      const toggle = page.locator(".sidebar-toggle");
      await expect(toggle).toBeVisible();
    });

    test("clicking hamburger opens sidebar", async ({ page }) => {
      const toggle = page.locator(".sidebar-toggle");
      await toggle.click();

      const sidebar = page.locator(".sidebar");
      await expect(sidebar).toHaveClass(/open/);
      await expect(sidebar).toBeInViewport();
    });

    test("backdrop appears when sidebar is open", async ({ page }) => {
      const toggle = page.locator(".sidebar-toggle");
      await toggle.click();

      const backdrop = page.locator(".sidebar-backdrop");
      await expect(backdrop).toHaveClass(/open/);
      await expect(backdrop).toBeVisible();
    });

    test("clicking backdrop closes sidebar", async ({ page }) => {
      const toggle = page.locator(".sidebar-toggle");
      await toggle.click();

      const backdrop = page.locator(".sidebar-backdrop");
      await expect(backdrop).toBeVisible();
      await backdrop.click();

      const sidebar = page.locator(".sidebar");
      await expect(sidebar).not.toHaveClass(/open/);
    });

    test("map container is full-width", async ({ page }) => {
      const map = page.locator(".map-container");
      const box = await map.boundingBox();
      expect(box).toBeTruthy();
      // Map should fill the viewport width (within a small margin)
      expect(box!.width).toBeGreaterThanOrEqual(370);
    });

    test("Escape key closes open sidebar", async ({ page }) => {
      const toggle = page.locator(".sidebar-toggle");
      await toggle.click();

      const sidebar = page.locator(".sidebar");
      await expect(sidebar).toHaveClass(/open/);

      await page.keyboard.press("Escape");
      await expect(sidebar).not.toHaveClass(/open/);
    });
  });

  test.describe("Toolbar buttons", () => {
    test("renders 4 toolbar buttons", async ({ page }) => {
      const buttons = page.locator(".toolbar-actions .toolbar-btn");
      await expect(buttons).toHaveCount(4);
    });

    test("undo and redo buttons are disabled when stacks are empty", async ({
      page,
    }) => {
      const undoBtn = page.locator(".toolbar-actions .toolbar-btn").nth(0);
      const redoBtn = page.locator(".toolbar-actions .toolbar-btn").nth(1);
      await expect(undoBtn).toBeDisabled();
      await expect(redoBtn).toBeDisabled();
    });

    test("delete button is disabled when no marker is selected", async ({
      page,
    }) => {
      const deleteBtn = page.locator(".toolbar-actions .toolbar-btn").nth(2);
      await expect(deleteBtn).toBeDisabled();
    });

    test("undo removes a placed gun marker", async ({ page }) => {
      const mapContainer = page.locator(".map-container");
      await expect(mapContainer).toBeVisible({ timeout: 10_000 });
      const box = await mapContainer.boundingBox();
      expect(box).toBeTruthy();

      // Place a gun marker
      await mapContainer.click({
        position: { x: box!.width / 2, y: box!.height / 2 },
      });
      const gunMarker = page.locator('.map-container svg text:text("GUN")');
      await expect(gunMarker).toBeVisible({ timeout: 5000 });

      // Undo button should now be enabled
      const undoBtn = page.locator(".toolbar-actions .toolbar-btn").nth(0);
      await expect(undoBtn).toBeEnabled();

      // Click undo
      await undoBtn.click();

      // Gun marker should be removed
      await expect(gunMarker).not.toBeVisible({ timeout: 5000 });
    });

    test("redo restores an undone gun marker", async ({ page }) => {
      const mapContainer = page.locator(".map-container");
      await expect(mapContainer).toBeVisible({ timeout: 10_000 });
      const box = await mapContainer.boundingBox();
      expect(box).toBeTruthy();

      // Place a gun marker
      await mapContainer.click({
        position: { x: box!.width / 2, y: box!.height / 2 },
      });
      const gunMarker = page.locator('.map-container svg text:text("GUN")');
      await expect(gunMarker).toBeVisible({ timeout: 5000 });

      // Undo
      const undoBtn = page.locator(".toolbar-actions .toolbar-btn").nth(0);
      await undoBtn.click();
      await expect(gunMarker).not.toBeVisible({ timeout: 5000 });

      // Redo button should now be enabled
      const redoBtn = page.locator(".toolbar-actions .toolbar-btn").nth(1);
      await expect(redoBtn).toBeEnabled();

      // Click redo
      await redoBtn.click();

      // Gun marker should reappear
      await expect(gunMarker).toBeVisible({ timeout: 5000 });
    });

    test("reset view button resets zoom", async ({ page }) => {
      const mapContainer = page.locator(".map-container");
      await expect(mapContainer).toBeVisible({ timeout: 10_000 });

      // Zoom in with scroll wheel
      await mapContainer.hover();
      await page.mouse.wheel(0, -300);
      await page.waitForTimeout(300);

      // Verify zoomed (transform scale > 1)
      const inner = page.locator(".map-inner");
      const transformBefore = await inner.evaluate(
        (el) => getComputedStyle(el).transform,
      );
      expect(transformBefore).not.toBe("none");

      // Click reset button (4th toolbar button)
      const resetBtn = page.locator(".toolbar-actions .toolbar-btn").nth(3);
      await resetBtn.click();
      await page.waitForTimeout(300);

      // Transform should be back to identity / scale(1)
      const transformAfter = await inner.evaluate(
        (el) => getComputedStyle(el).transform,
      );
      // After reset, transform is "translate(0px, 0px) scale(1)" or "none" or matrix(1,0,0,1,0,0)
      const isReset =
        transformAfter === "none" ||
        transformAfter.includes("matrix(1, 0, 0, 1, 0, 0)");
      expect(isReset).toBe(true);
    });

    test("toolbar is visible at mobile viewport", async ({ page }) => {
      await page.setViewportSize({ width: 375, height: 667 });
      await page.waitForTimeout(300);

      const toolbar = page.locator(".toolbar-actions");
      await expect(toolbar).toBeVisible();

      const buttons = page.locator(".toolbar-actions .toolbar-btn");
      await expect(buttons).toHaveCount(4);
    });

    test("all header buttons fit within viewport on iPhone SE (375px)", async ({
      page,
    }) => {
      await page.setViewportSize({ width: 375, height: 667 });
      await page.waitForTimeout(300);

      // Header should not overflow the viewport
      const overflows = await page.evaluate(() => {
        const header = document.querySelector(".header") as HTMLElement;
        return header.scrollWidth > header.offsetWidth;
      });
      expect(overflows).toBe(false);

      // Every toolbar button should be fully within the viewport
      const toolbarBtns = page.locator(".toolbar-actions .toolbar-btn");
      for (let i = 0; i < 4; i++) {
        const box = await toolbarBtns.nth(i).boundingBox();
        expect(box).toBeTruthy();
        expect(box!.x).toBeGreaterThanOrEqual(0);
        expect(box!.x + box!.width).toBeLessThanOrEqual(375);
      }

      // Every placement mode button should be fully within the viewport
      const placementBtns = page.locator(".placement-mode button");
      for (let i = 0; i < 3; i++) {
        const box = await placementBtns.nth(i).boundingBox();
        expect(box).toBeTruthy();
        expect(box!.x).toBeGreaterThanOrEqual(0);
        expect(box!.x + box!.width).toBeLessThanOrEqual(375);
      }

      // Faction toggle buttons should be fully within the viewport
      const factionBtns = page.locator(".faction-toggle button");
      for (let i = 0; i < 2; i++) {
        const box = await factionBtns.nth(i).boundingBox();
        expect(box).toBeTruthy();
        expect(box!.x).toBeGreaterThanOrEqual(0);
        expect(box!.x + box!.width).toBeLessThanOrEqual(375);
      }
    });
  });
});

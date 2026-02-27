import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "./e2e",
  timeout: 30_000,
  retries: 0,
  use: {
    baseURL: "http://localhost:8080",
    headless: true,
    screenshot: "only-on-failure",
  },
  projects: [
    {
      name: "chromium",
      use: { browserName: "chromium" },
    },
  ],
  webServer: [
    {
      command: "cargo run -p foxhole-backend",
      port: 3000,
      timeout: 120_000,
      reuseExistingServer: true,
    },
    {
      command: "cd crates/frontend && dx serve",
      port: 8080,
      timeout: 120_000,
      reuseExistingServer: true,
    },
  ],
});

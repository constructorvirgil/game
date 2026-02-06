import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "./e2e",
  fullyParallel: false,
  workers: 1,
  timeout: 60_000,
  expect: {
    timeout: 10_000,
  },
  reporter: [["list"], ["html", { open: "never" }]],
  use: {
    baseURL: "http://127.0.0.1:5173",
    trace: "retain-on-failure",
    screenshot: "only-on-failure",
    video: "retain-on-failure",
  },
  webServer: [
    {
      command: "node e2e/start-backend.mjs",
      url: "http://127.0.0.1:33030/health",
      timeout: 120_000,
      reuseExistingServer: true,
    },
    {
      command: "node e2e/static-server.mjs",
      url: "http://127.0.0.1:5173",
      timeout: 120_000,
      reuseExistingServer: true,
    },
  ],
});

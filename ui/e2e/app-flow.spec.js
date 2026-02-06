import { expect, test } from "@playwright/test";

const TEST_WS_URL = "ws://127.0.0.1:33030/ws";

async function waitConnected(page) {
  await expect
    .poll(async () => page.locator("#status").getAttribute("data-tone"), { timeout: 20_000 })
    .toBe("ok");
}

async function openWithLocalWs(page) {
  await page.addInitScript((url) => {
    window.localStorage.setItem("ddz.wsUrl", url);
  }, TEST_WS_URL);
  await page.goto("/");
}

async function joinRoomFromList(page, roomId) {
  const item = page.locator(".room-item", { hasText: roomId }).first();
  await expect(item).toBeVisible({ timeout: 20_000 });
  await item.click();
  await expect(page.locator("#roomId")).toHaveText(roomId, { timeout: 20_000 });
}

test("first entry should not show game over modal", async ({ page }) => {
  await openWithLocalWs(page);
  await waitConnected(page);
  await expect(page.locator("#gameOverModal")).toBeHidden();
});

test("after 3 players joined and game started, game over modal stays hidden", async ({ browser }) => {
  const ownerCtx = await browser.newContext();
  const p2Ctx = await browser.newContext();
  const p3Ctx = await browser.newContext();
  const owner = await ownerCtx.newPage();
  const p2 = await p2Ctx.newPage();
  const p3 = await p3Ctx.newPage();

  try {
    await openWithLocalWs(owner);
    await waitConnected(owner);
    await owner.click("#createRoomBtn");

    const ownerRoom = owner.locator("#roomId");
    await expect
      .poll(async () => (await ownerRoom.textContent())?.trim() || "", { timeout: 20_000 })
      .not.toBe("-");
    const roomId = ((await ownerRoom.textContent()) || "").trim();

    await openWithLocalWs(p2);
    await waitConnected(p2);
    await joinRoomFromList(p2, roomId);

    await openWithLocalWs(p3);
    await waitConnected(p3);
    await joinRoomFromList(p3, roomId);

    await expect(owner.locator("#myRole")).not.toHaveText("-", { timeout: 20_000 });
    await expect(owner.locator("#turnBanner")).not.toContainText("等待玩家加入", { timeout: 20_000 });

    await expect(owner.locator("#gameOverModal")).toBeHidden();
    await expect(p2.locator("#gameOverModal")).toBeHidden();
    await expect(p3.locator("#gameOverModal")).toBeHidden();
  } finally {
    await p3Ctx.close();
    await p2Ctx.close();
    await ownerCtx.close();
  }
});

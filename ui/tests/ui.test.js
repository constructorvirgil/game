import { beforeEach, describe, expect, it } from "vitest";
import {
  applyRoomState,
  clearSelection,
  formatPlay,
  handleServerMessage,
  renderHand,
  renderRoomList,
  selectedCards,
  state,
} from "../app.js";

function setupDom() {
  document.body.innerHTML = `
    <div id="status" data-tone="idle"></div>
    <input id="serverUrl" value="ws://127.0.0.1:3030/ws" />
    <button id="reconnectBtn"></button>
    <button id="createRoomBtn"></button>
    <button id="joinRoomBtn"></button>
    <button id="refreshRoomsBtn"></button>
    <button id="playBtn"></button>
    <button id="passBtn"></button>
    <button id="clearBtn"></button>
    <button id="restartGameBtn"></button>
    <input id="roomIdInput" readonly />
    <div id="userId"></div>
    <div id="roomId"></div>
    <div id="players"></div>
    <div id="roomList"></div>
    <div id="leftSeat"></div>
    <div id="rightSeat"></div>
    <div id="myRole"></div>
    <div id="myHandCount"></div>
    <div id="selectedCount"></div>
    <div id="turnBanner"></div>
    <div id="hand"></div>
    <div id="lastPlay"></div>
    <div id="lastPlayer"></div>
    <div id="messages"></div>
    <div id="recommendations"></div>
    <div id="gameOverModal" hidden></div>
    <div id="gameOverWinner"></div>
    <div id="gameOverHint"></div>
  `;
}

beforeEach(() => {
  setupDom();
  state.userId = 1;
  state.userName = "Brave_Panda";
  state.roomId = null;
  state.selectedRoomId = null;
  state.currentRoomPlayerCount = 0;
  state.roomStarted = false;
  state.players = [];
  state.hand = [];
  state.turn = null;
  state.connected = false;
  state.serverUrl = "ws://127.0.0.1:33030/ws";
  state.roomList = [];
  state.recommendations = [];
  state.gameOverWinnerId = null;
  state.cardDiagEnabled = false;
  state.playerNames = new Map([[1, "Brave_Panda"]]);
});

describe("ui rendering", () => {
  it("keeps game over modal hidden on first enter", () => {
    expect(state.gameOverWinnerId).toBe(null);
    expect(document.getElementById("gameOverModal").hidden).toBe(true);
  });

  it("renders hand cards", () => {
    renderHand(document.getElementById("hand"), ["S3", "H4", "BJ"]);
    const cards = document.querySelectorAll(".card");
    expect(cards.length).toBe(3);
    expect(cards[1].textContent).toContain("红桃4");
    const image = cards[0].querySelector(".card-image");
    expect(image.src).toContain("http://127.0.0.1:33030/assets/cards/3S.png");
    expect(image.src).not.toContain("deckofcardsapi.com");
  });

  it("collects and clears selected cards", () => {
    renderHand(document.getElementById("hand"), ["S3", "H4"]);
    document.querySelectorAll(".card")[1].click();
    expect(selectedCards()).toEqual(["H4"]);
    clearSelection();
    expect(selectedCards()).toEqual([]);
  });

  it("formats play display", () => {
    expect(formatPlay(null)).toBe("暂无出牌");
    expect(formatPlay({ kind: "Single", main_rank: "Three", size: 1 })).toContain("单张");
  });

  it("applies room snapshot and updates turn banner", () => {
    state.connected = true;
    applyRoomState({
      room_id: "ABC123",
      players: [
        { id: 1, name: "Brave_Panda", hand_count: 17, is_landlord: true },
        { id: 2, name: "Calm_Tiger", hand_count: 17, is_landlord: false },
        { id: 3, name: "Swift_Wolf", hand_count: 17, is_landlord: false },
      ],
      turn: 1,
      last_player: 2,
      last_play: { kind: "Single", main_rank: "Three", size: 1 },
      your_hand: ["S3", "H4"],
    });

    expect(state.roomId).toBe("ABC123");
    expect(document.getElementById("turnBanner").textContent).toContain("轮到你出牌");
    expect(document.getElementById("recommendations").children.length).toBeGreaterThan(0);
  });

  it("shows waiting status after joined not started", () => {
    state.connected = true;
    handleServerMessage({
      type: "Joined",
      data: {
        room_id: "XYZ888",
        you: 1,
        you_name: "Brave_Panda",
        player_count: 1,
        started: false,
      },
    });

    expect(document.getElementById("turnBanner").textContent).toContain("等待玩家加入");
    expect(document.getElementById("roomIdInput").value).toBe("XYZ888");
  });

  it("renders room list", () => {
    state.connected = true;
    renderRoomList(document.getElementById("roomList"), [
      { room_id: "A11111", player_count: 1, started: false, can_join: true },
      { room_id: "B22222", player_count: 3, started: true, can_join: false },
    ]);

    expect(document.querySelectorAll(".room-item").length).toBe(2);
    expect(document.getElementById("roomList").textContent).toContain("A11111");
  });

  it("shows and hides game over modal", () => {
    state.connected = true;
    state.roomId = "R00001";
    state.roomStarted = true;
    state.playerNames.set(2, "Calm_Tiger");

    handleServerMessage({
      type: "GameOver",
      data: { room_id: "R00001", winner_id: 2 },
    });

    expect(state.gameOverWinnerId).toBe(2);
    expect(document.getElementById("gameOverModal").hidden).toBe(false);
    expect(document.getElementById("gameOverWinner").textContent).toContain("Calm_Tiger");

    handleServerMessage({
      type: "GameRestarted",
      data: { room_id: "R00001" },
    });

    expect(state.gameOverWinnerId).toBe(null);
    expect(document.getElementById("gameOverModal").hidden).toBe(true);
  });

  it("does not infer game over from room snapshot", () => {
    state.connected = true;
    applyRoomState({
      room_id: "ABC123",
      players: [
        { id: 1, name: "Brave_Panda", hand_count: 17, is_landlord: true },
        { id: 2, name: "Calm_Tiger", hand_count: 0, is_landlord: false },
        { id: 3, name: "Swift_Wolf", hand_count: 17, is_landlord: false },
      ],
      turn: 1,
      last_player: 2,
      last_play: { kind: "Single", main_rank: "Three", size: 1 },
      your_hand: ["S3", "H4"],
    });

    expect(state.gameOverWinnerId).toBe(null);
    expect(document.getElementById("gameOverModal").hidden).toBe(true);
  });

  it("ignores game over from another room", () => {
    state.connected = true;
    state.roomId = "ROOM_A";
    state.selectedRoomId = "ROOM_A";
    state.roomStarted = true;

    handleServerMessage({
      type: "GameOver",
      data: { room_id: "ROOM_B", winner_id: 2 },
    });

    expect(state.gameOverWinnerId).toBe(null);
    expect(document.getElementById("gameOverModal").hidden).toBe(true);
  });

  it("ignores game over without room id", () => {
    state.connected = true;
    state.roomId = "ROOM_A";
    state.roomStarted = true;

    handleServerMessage({
      type: "GameOver",
      data: { winner_id: 2 },
    });

    expect(state.gameOverWinnerId).toBe(null);
    expect(document.getElementById("gameOverModal").hidden).toBe(true);
  });

  it("clears stale game over when active room state arrives", () => {
    state.connected = true;
    state.roomId = "ROOM_A";
    state.roomStarted = true;

    handleServerMessage({
      type: "GameOver",
      data: { room_id: "ROOM_A", winner_id: 2 },
    });
    expect(document.getElementById("gameOverModal").hidden).toBe(false);

    applyRoomState({
      room_id: "ROOM_A",
      players: [
        { id: 1, name: "Brave_Panda", hand_count: 17, is_landlord: true },
        { id: 2, name: "Calm_Tiger", hand_count: 17, is_landlord: false },
        { id: 3, name: "Swift_Wolf", hand_count: 20, is_landlord: false },
      ],
      turn: 1,
      last_player: null,
      last_play: null,
      your_hand: ["S3", "H4"],
    });

    expect(state.gameOverWinnerId).toBe(null);
    expect(document.getElementById("gameOverModal").hidden).toBe(true);
  });

  it("marks round ended when room is interrupted by leaver", () => {
    state.connected = true;
    state.roomId = "ROOM_A";
    state.selectedRoomId = "ROOM_A";
    state.roomStarted = true;
    state.playerNames.set(2, "Calm_Tiger");
    state.hand = ["S3", "H4"];
    renderHand(document.getElementById("hand"), state.hand);

    handleServerMessage({
      type: "RoomInterrupted",
      data: { room_id: "ROOM_A", leaver_id: 2, player_count: 2 },
    });

    expect(state.roomStarted).toBe(false);
    expect(state.currentRoomPlayerCount).toBe(2);
    expect(state.hand).toEqual([]);
    expect(document.getElementById("turnBanner").textContent).toContain("等待玩家加入");
    expect(document.getElementById("lastPlay").textContent).toContain("玩家离开");
  });
});

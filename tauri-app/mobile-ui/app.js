const DEFAULT_WS_URL = "ws://100.70.102.165:33030/ws";
const CARD_ASSET_PATH = "assets/cards";
const ROOM_LIST_POLL_MS = 500;
const MAX_LOG_LINES = 24;
const MAX_RECOMMEND = 5;

const KIND_LABEL = {
  Single: "单张",
  Pair: "对子",
  Triple: "三张",
  TripleSingle: "三带一",
  TriplePair: "三带二",
  Straight: "顺子",
  DoubleStraight: "连对",
  Airplane: "飞机",
  Bomb: "炸弹",
  Rocket: "王炸",
  FourTwoSingle: "四带二",
  FourTwoPair: "四带两对",
};

const RANK_VALUE = {
  "3": 3,
  "4": 4,
  "5": 5,
  "6": 6,
  "7": 7,
  "8": 8,
  "9": 9,
  "10": 10,
  J: 11,
  Q: 12,
  K: 13,
  A: 14,
  "2": 16,
  Three: 3,
  Four: 4,
  Five: 5,
  Six: 6,
  Seven: 7,
  Eight: 8,
  Nine: 9,
  Ten: 10,
  Jack: 11,
  Queen: 12,
  King: 13,
  Ace: 14,
  Two: 16,
  BJ: 17,
  RJ: 18,
  BlackJoker: 17,
  RedJoker: 18,
};

const RANK_LABEL = {
  3: "3",
  4: "4",
  5: "5",
  6: "6",
  7: "7",
  8: "8",
  9: "9",
  10: "10",
  11: "J",
  12: "Q",
  13: "K",
  14: "A",
  16: "2",
  17: "小王",
  18: "大王",
};

const SUIT_LABEL = {
  S: "黑桃",
  H: "红桃",
  C: "梅花",
  D: "方块",
};

const SUIT_ORDER = {
  C: 1,
  D: 2,
  H: 3,
  S: 4,
  J: 5,
};

export const state = {
  ws: null,
  connected: false,
  userId: null,
  userName: "-",
  roomId: null,
  selectedRoomId: null,
  roomStarted: false,
  currentRoomPlayerCount: 0,
  players: [],
  hand: [],
  lastPlay: null,
  lastPlayer: null,
  turn: null,
  roomList: [],
  playerNames: new Map(),
  recommendations: [],
  gameOverWinnerId: null,
  cardDiagEnabled: true,
  serverUrl: DEFAULT_WS_URL,
  reconnectAttempts: 0,
  reconnectTimer: null,
  roomListTimer: null,
};

const nodes = {};
let initialized = false;
let lastHandLayoutLog = "";

function el(id) {
  const cached = nodes[id];
  if (cached && typeof document !== "undefined" && document.contains(cached)) return cached;
  if (typeof document === "undefined") return null;
  const node = document.getElementById(id);
  if (node) nodes[id] = node;
  return node;
}

function setText(id, text) {
  const node = el(id);
  if (node) node.textContent = text;
}

function readUrl() {
  if (typeof localStorage === "undefined") return DEFAULT_WS_URL;
  return localStorage.getItem("ddz.wsUrl") || DEFAULT_WS_URL;
}

function writeUrl(url) {
  if (typeof localStorage !== "undefined") {
    localStorage.setItem("ddz.wsUrl", url);
  }
}

function parseCard(code) {
  if (!code) return null;
  if (code === "BJ") {
    return { code, suit: "J", rank: 17, label: "小王", image: "X1", tone: "joker" };
  }
  if (code === "RJ") {
    return { code, suit: "J", rank: 18, label: "大王", image: "X2", tone: "joker" };
  }

  const suit = code[0];
  const rankToken = code.slice(1);
  const rank = RANK_VALUE[rankToken];
  if (!SUIT_LABEL[suit] || !rank) return null;

  const imageRank = rankToken === "10" ? "0" : rankToken;
  const tone = suit === "H" || suit === "D" ? "red" : "black";
  return {
    code,
    suit,
    rank,
    label: `${SUIT_LABEL[suit]}${RANK_LABEL[rank]}`,
    image: `${imageRank}${suit}`,
    tone,
  };
}

function isTauriRuntime() {
  if (typeof window === "undefined") return false;
  const ua = typeof navigator !== "undefined" ? navigator.userAgent || "" : "";
  return Boolean(window.__TAURI_INTERNALS__ || window.__TAURI__ || window.location?.protocol === "tauri:" || ua.includes("Tauri"));
}

function backendBaseFromWsUrl(wsUrl) {
  try {
    const parsed = new URL(wsUrl);
    const protocol = parsed.protocol === "wss:" ? "https:" : "http:";
    return `${protocol}//${parsed.host}`;
  } catch {
    return "";
  }
}

function cardAssetBaseUrl() {
  if (isTauriRuntime()) return CARD_ASSET_PATH;
  const backendBase = backendBaseFromWsUrl(state.serverUrl || DEFAULT_WS_URL);
  return backendBase ? `${backendBase}/${CARD_ASSET_PATH}` : CARD_ASSET_PATH;
}

function cardBackImageUrl() {
  return `${cardAssetBaseUrl()}/back.png`;
}

function cardImage(code) {
  const card = parseCard(code);
  return card ? `${cardAssetBaseUrl()}/${card.image}.png` : cardBackImageUrl();
}

function rankValue(token) {
  return RANK_VALUE[token] ?? Number(token) ?? null;
}

function rankLabel(value) {
  return RANK_LABEL[value] || String(value);
}

function rankToLabel(token) {
  const value = rankValue(token);
  return value ? rankLabel(value) : String(token);
}

function sortHand(codes) {
  return [...codes].sort((a, b) => {
    const ca = parseCard(a);
    const cb = parseCard(b);
    if (!ca || !cb) return a.localeCompare(b);
    if (cb.rank !== ca.rank) return cb.rank - ca.rank;
    return (SUIT_ORDER[cb.suit] || 0) - (SUIT_ORDER[ca.suit] || 0);
  });
}

function saveName(id, name) {
  if (id === null || id === undefined) return;
  const fallback = `Player_${String(id).slice(-4)}`;
  state.playerNames.set(id, name || fallback);
}

function nameById(id) {
  if (id === null || id === undefined) return "玩家";
  return state.playerNames.get(id) || `Player_${String(id).slice(-4)}`;
}

function setStatus(text, tone = "idle") {
  const node = el("status");
  if (!node) return;
  node.textContent = text;
  node.dataset.tone = tone;
}

function logMessage(text) {
  const container = el("messages");
  if (!container) return;
  const now = new Date();
  const line = document.createElement("div");
  line.className = "msg";
  line.textContent = `[${String(now.getHours()).padStart(2, "0")}:${String(now.getMinutes()).padStart(2, "0")}:${String(now.getSeconds()).padStart(2, "0")}] ${text}`;
  container.prepend(line);
  while (container.children.length > MAX_LOG_LINES) {
    container.removeChild(container.lastElementChild);
  }
}

function isGameOver() {
  return state.gameOverWinnerId !== null && state.gameOverWinnerId !== undefined;
}

function setGameOver(winnerId, source = "server") {
  state.gameOverWinnerId = winnerId ?? null;
  const over = isGameOver();

  const modal = el("gameOverModal");
  if (modal) modal.hidden = !over;

  if (over) {
    setText("gameOverWinner", `胜者：${nameById(state.gameOverWinnerId)}`);
    const hint = el("gameOverHint");
    if (hint) {
      hint.textContent = `本局结束（${source}），可以发起再来一局。`;
    }
  }

  const restartBtn = el("restartGameBtn");
  if (restartBtn) {
    restartBtn.disabled = !over || !state.connected || !state.roomId;
  }
}

function logCardDiagnostics(reason) {
  if (!state.cardDiagEnabled || typeof document === "undefined" || typeof window === "undefined") return;
  const cards = Array.from(document.querySelectorAll("#hand .card"));
  if (!cards.length) return;

  const selected = cards.filter((card) => card.classList.contains("selected"));
  const detail = selected
    .map((card) => {
      const idx = cards.indexOf(card);
      const style = window.getComputedStyle(card);
      const rect = card.getBoundingClientRect();
      const left = cards[idx - 1];
      const right = cards[idx + 1];
      const overlap = (other) => {
        if (!other) return 0;
        const or = other.getBoundingClientRect();
        return Math.max(0, Math.min(rect.right, or.right) - Math.max(rect.left, or.left));
      };
      return `${card.dataset.code}@${idx} z=${style.zIndex} top=${style.top} overlap(L:${Math.round(overlap(left))},R:${Math.round(overlap(right))})`;
    })
    .join(" | ");

  logMessage(`[CARD-DIAG] ${reason}; selected=${selected.length}; ${detail || "none"}`);
}

function handLayoutProfile() {
  const mobile = typeof window !== "undefined" && window.matchMedia?.("(max-width: 720px)")?.matches;
  return mobile
    ? { baseWidth: 70, baseStep: 40, minWidth: 50, minVisible: 16, hardMinWidth: 42, hardMinVisible: 10 }
    : { baseWidth: 82, baseStep: 48, minWidth: 58, minVisible: 18, hardMinWidth: 48, hardMinVisible: 12 };
}

function syncHandLayout(container = el("hand")) {
  if (!container || typeof window === "undefined") return;
  const cardCount = container.querySelectorAll(".card").length;
  if (cardCount <= 0) {
    container.style.removeProperty("--card-width");
    container.style.removeProperty("--card-step");
    return;
  }

  const profile = handLayoutProfile();
  const computed = window.getComputedStyle(container);
  const padLeft = Number.parseFloat(computed.paddingLeft) || 0;
  const padRight = Number.parseFloat(computed.paddingRight) || 0;
  const available = Math.max(0, container.clientWidth - padLeft - padRight);
  if (!available) return;

  let width = profile.baseWidth;
  let step = profile.baseStep;

  if (cardCount > 1) {
    width = Math.min(
      profile.baseWidth,
      Math.max(profile.minWidth, available - (cardCount - 1) * profile.minVisible),
    );
    step = Math.min(profile.baseStep, width);

    let total = width + (cardCount - 1) * step;
    if (total > available) {
      step = Math.max(profile.minVisible, (available - width) / (cardCount - 1));
      total = width + (cardCount - 1) * step;
    }
    if (total > available) {
      width = Math.max(
        profile.hardMinWidth,
        Math.min(profile.baseWidth, available - (cardCount - 1) * profile.hardMinVisible),
      );
      step = Math.max(profile.hardMinVisible, (available - width) / (cardCount - 1));
    }

    step = Math.max(profile.hardMinVisible, Math.min(step, width));
    width = Math.max(profile.hardMinWidth, Math.min(width, profile.baseWidth));
  }

  const widthPx = `${Math.round(width)}px`;
  const stepPx = `${Number(step.toFixed(2))}px`;
  container.style.setProperty("--card-width", widthPx);
  container.style.setProperty("--card-step", stepPx);

  if (state.cardDiagEnabled) {
    const signature = `${cardCount}|${Math.round(available)}|${widthPx}|${stepPx}`;
    if (signature !== lastHandLayoutLog) {
      lastHandLayoutLog = signature;
      logMessage(`[CARD-LAYOUT] count=${cardCount} width=${widthPx} step=${stepPx} available=${Math.round(available)}px`);
    }
  }
}

function clearReconnect() {
  if (state.reconnectTimer !== null && typeof window !== "undefined") {
    window.clearTimeout(state.reconnectTimer);
  }
  state.reconnectTimer = null;
}

function stopRoomPolling() {
  if (state.roomListTimer !== null && typeof window !== "undefined") {
    window.clearInterval(state.roomListTimer);
  }
  state.roomListTimer = null;
}

function startRoomPolling() {
  if (typeof window === "undefined" || state.roomListTimer !== null) return;
  state.roomListTimer = window.setInterval(requestRoomList, ROOM_LIST_POLL_MS);
}

function scheduleReconnect() {
  if (typeof window === "undefined" || state.reconnectTimer !== null) return;
  const delay = Math.min(12000, 1500 * 2 ** state.reconnectAttempts);
  state.reconnectAttempts += 1;
  setStatus(`连接中断，${Math.ceil(delay / 1000)} 秒后重连`, "warn");
  logMessage("连接中断，准备自动重连...");
  state.reconnectTimer = window.setTimeout(() => {
    state.reconnectTimer = null;
    connect(state.serverUrl, { retry: true });
  }, delay);
}

function syncRoomInput() {
  const input = el("roomIdInput");
  if (input) input.value = state.roomId || state.selectedRoomId || "";
}

function resetRoom() {
  state.roomId = null;
  state.selectedRoomId = null;
  state.roomStarted = false;
  state.currentRoomPlayerCount = 0;
  state.players = [];
  state.hand = [];
  state.lastPlay = null;
  state.lastPlayer = null;
  state.turn = null;
  state.recommendations = [];
  setText("roomId", "-");
  setText("myRole", "-");
  setText("myHandCount", "0 张");
  setText("selectedCount", "0 张");
  setText("lastPlay", "暂无出牌");
  setText("lastPlayer", "本轮还没有人出牌");
  syncRoomInput();
  setGameOver(null, "reset");
}

function requestRoomList() {
  if (state.connected) sendMessage({ type: "ListRooms" });
}

function joinRoom(roomId) {
  if (!roomId) return;
  if (!state.connected) return logMessage("尚未连接服务器");
  state.selectedRoomId = roomId;
  syncRoomInput();
  sendMessage({ type: "JoinRoom", data: { room_id: roomId } });
}

export function connect(url, options = {}) {
  if (!url) return;
  state.serverUrl = url;
  writeUrl(url);
  if (!options.retry) state.reconnectAttempts = 0;

  clearReconnect();
  if (state.ws) {
    state.ws.__manualClose = true;
    state.ws.close();
  }

  const ws = new WebSocket(url);
  ws.__manualClose = false;
  state.ws = ws;
  setStatus("连接中...", "pending");

  ws.addEventListener("open", () => {
    if (state.ws !== ws) return;
    state.connected = true;
    state.reconnectAttempts = 0;
    resetRoom();
    setStatus("已连接", "ok");
    logMessage("WebSocket 已连接");
    startRoomPolling();
    updateTurnBanner();
    updateActionState();
    requestRoomList();
  });

  ws.addEventListener("close", () => {
    if (state.ws === ws) state.ws = null;
    state.connected = false;
    stopRoomPolling();
    updateActionState();
    updateTurnBanner();
    if (ws.__manualClose) {
      setStatus("已断开", "idle");
      return;
    }
    scheduleReconnect();
  });

  ws.addEventListener("error", () => logMessage("连接异常，准备重连"));
  ws.addEventListener("message", (event) => {
    try {
      handleServerMessage(JSON.parse(event.data));
    } catch (_error) {
      logMessage("消息解析失败");
    }
  });
}

export function sendMessage(payload) {
  if (!state.ws || state.ws.readyState !== WebSocket.OPEN) return false;
  state.ws.send(JSON.stringify(payload));
  return true;
}

function buildSeats(players) {
  if (!players || players.length === 0) {
    return { left: null, right: null, self: null };
  }
  const selfIndex = players.findIndex((player) => player.id === state.userId);
  const base = selfIndex >= 0 ? selfIndex : 0;
  return {
    self: players[base] || null,
    right: players.length > 1 ? players[(base + 1) % players.length] : null,
    left: players.length > 2 ? players[(base + 2) % players.length] : null,
  };
}

function playerLabel(player) {
  return player ? player.name || nameById(player.id) : "玩家";
}

function renderOpponentSeat(container, player, turnId, sideLabel) {
  if (!container) return;
  container.innerHTML = "";
  container.className = "seat opponent-seat";

  if (!player) {
    container.classList.add("waiting");
    const empty = document.createElement("div");
    empty.className = "seat-empty";
    empty.textContent = `${sideLabel}：等待玩家加入`;
    container.appendChild(empty);
    return;
  }

  if (player.id === turnId) {
    container.classList.add("active");
  }

  const title = document.createElement("div");
  title.className = "seat-title";
  title.textContent = playerLabel(player);

  const tags = document.createElement("div");
  tags.className = "seat-tags";

  if (player.is_landlord) {
    const landlord = document.createElement("span");
    landlord.className = "tag landlord";
    landlord.textContent = "地主";
    tags.appendChild(landlord);
  }

  if (player.id === turnId) {
    const turn = document.createElement("span");
    turn.className = "tag turn";
    turn.textContent = "当前出牌";
    tags.appendChild(turn);
  }

  const stack = document.createElement("div");
  stack.className = "card-stack";
  const count = Math.max(2, Math.min(7, Math.ceil(player.hand_count / 3)));
  for (let i = 0; i < count; i += 1) {
    const back = document.createElement("img");
    back.className = "card-back";
    back.src = cardBackImageUrl();
    back.alt = "对手手牌";
    back.loading = "lazy";
    back.style.setProperty("--i", String(i));
    stack.appendChild(back);
  }

  const remain = document.createElement("div");
  remain.className = "seat-count";
  remain.textContent = `剩余 ${player.hand_count} 张`;

  container.appendChild(title);
  container.appendChild(tags);
  container.appendChild(stack);
  container.appendChild(remain);
}

function describeLastPlayer(playerId) {
  if (!playerId) return "本轮还没有人出牌";
  if (playerId === state.userId) return "上一手出牌：你";
  return `上一手出牌：${nameById(playerId)}`;
}

function selectedCardsSet() {
  return new Set(selectedCards());
}

function updateSelectedCount() {
  setText("selectedCount", `${selectedCards().length} 张`);
}

function classifyRoomStatus(room) {
  if (room.started) return "进行中";
  if (room.player_count >= 3) return "已满";
  return "可加入";
}

export function renderRoomList(container, rooms) {
  if (!container) return;
  container.innerHTML = "";

  if (!rooms || rooms.length === 0) {
    const empty = document.createElement("div");
    empty.className = "room-empty";
    empty.textContent = "暂无可用房间，点击“创建房间”开始。";
    container.appendChild(empty);
    return;
  }

  rooms.forEach((room) => {
    const item = document.createElement("button");
    item.type = "button";
    item.className = "room-item";
    if (room.room_id === state.roomId) item.classList.add("current");
    if (room.room_id === state.selectedRoomId) item.classList.add("selected");
    if (!room.can_join && room.room_id !== state.roomId) item.classList.add("full");

    item.innerHTML = `<span class="room-id">${room.room_id}</span><span class="room-meta">${room.player_count}/3 · ${classifyRoomStatus(room)}</span>`;

    item.addEventListener("click", () => {
      state.selectedRoomId = room.room_id;
      syncRoomInput();
      if (room.can_join && room.room_id !== state.roomId) {
        joinRoom(room.room_id);
      }
      renderRoomList(container, rooms);
      updateActionState();
      updateTurnBanner();
    });

    container.appendChild(item);
  });
}

function groupByRank(handCodes) {
  const groups = new Map();
  handCodes
    .map(parseCard)
    .filter(Boolean)
    .forEach((card) => {
      if (!groups.has(card.rank)) groups.set(card.rank, []);
      groups.get(card.rank).push(card.code);
    });
  return groups;
}

function windows(values, size) {
  const result = [];
  for (let i = 0; i <= values.length - size; i += 1) {
    const slice = values.slice(i, i + size);
    let consecutive = true;
    for (let j = 1; j < slice.length; j += 1) {
      if (slice[j] !== slice[j - 1] + 1) consecutive = false;
    }
    if (consecutive) result.push(slice);
  }
  return result;
}

function beat(prev, next) {
  if (!prev || !next) return false;
  if (prev.kind === "Rocket") return false;
  if (next.kind === "Rocket") return true;
  if (next.kind === "Bomb" && prev.kind !== "Bomb") return true;
  if (prev.kind === "Bomb" && next.kind !== "Bomb") return false;
  if (prev.kind !== next.kind) return false;

  if (["Straight", "DoubleStraight", "Airplane"].includes(prev.kind)) {
    return prev.size === next.size && next.mainRank > prev.mainRank;
  }
  return next.mainRank > prev.mainRank;
}

function recKey(rec) {
  return [...rec.codes].sort().join(",");
}

function buildRecommendations() {
  if (isGameOver() || !state.hand.length) return [];

  const groups = groupByRank(state.hand);
  const ranks = [...groups.keys()].sort((a, b) => a - b);
  const prev = state.lastPlay && state.lastPlayer !== state.userId
    ? {
      kind: state.lastPlay.kind,
      mainRank: rankValue(state.lastPlay.main_rank),
      size: Number(state.lastPlay.size) || 0,
    }
    : null;

  const recs = [];
  const add = (rec) => {
    if (!rec || !rec.codes || rec.codes.some((code) => !code)) return;
    if (recs.find((item) => recKey(item) === recKey(rec))) return;
    recs.push(rec);
  };

  const singles = () => ranks.forEach((rank) => add({ kind: "Single", mainRank: rank, size: 1, codes: [groups.get(rank)[0]] }));
  const pairs = () => ranks.forEach((rank) => {
    if (groups.get(rank).length >= 2) add({ kind: "Pair", mainRank: rank, size: 2, codes: groups.get(rank).slice(0, 2) });
  });
  const triples = () => ranks.forEach((rank) => {
    if (groups.get(rank).length >= 3) add({ kind: "Triple", mainRank: rank, size: 3, codes: groups.get(rank).slice(0, 3) });
  });
  const bombs = () => ranks.forEach((rank) => {
    if (groups.get(rank).length === 4) add({ kind: "Bomb", mainRank: rank, size: 4, codes: groups.get(rank).slice(0, 4) });
  });
  const rocket = () => {
    if (groups.has(17) && groups.has(18)) {
      add({ kind: "Rocket", mainRank: 18, size: 2, codes: [groups.get(17)[0], groups.get(18)[0]] });
    }
  };

  if (!prev) {
    singles();
    pairs();
    triples();
    return recs.slice(0, MAX_RECOMMEND);
  }

  if (prev.kind === "Single") singles();
  if (prev.kind === "Pair") pairs();
  if (prev.kind === "Triple") triples();
  if (prev.kind === "Bomb") bombs();
  if (prev.kind !== "Rocket") {
    bombs();
    rocket();
  }

  if (prev.kind === "Straight" || prev.kind === "DoubleStraight" || prev.kind === "Airplane") {
    const need = prev.kind === "Straight" ? 1 : prev.kind === "DoubleStraight" ? 2 : 3;
    const chain = ranks.filter((rank) => rank >= 3 && rank <= 14 && groups.get(rank).length >= need);
    windows(chain, prev.size).forEach((windowRanks) => {
      const codes = windowRanks.flatMap((rank) => groups.get(rank).slice(0, need));
      add({ kind: prev.kind, mainRank: windowRanks[windowRanks.length - 1], size: prev.size, codes });
    });
    if (prev.kind !== "Rocket") {
      bombs();
      rocket();
    }
  }

  const filtered = recs.filter((rec) => beat(prev, rec));
  filtered.sort((a, b) => {
    const score = (kind) => {
      if (kind === prev.kind) return 0;
      if (kind === "Bomb") return 1;
      if (kind === "Rocket") return 2;
      return 3;
    };
    return score(a.kind) - score(b.kind) || a.mainRank - b.mainRank;
  });

  return filtered.slice(0, MAX_RECOMMEND);
}

function recLabel(rec) {
  const kind = KIND_LABEL[rec.kind] || rec.kind;
  const rank = rankLabel(rec.mainRank);
  const brief = rec.codes.slice(0, 4).map((code) => parseCard(code)?.label || code).join(" ");
  return `${kind}(${rank}) ${brief}`;
}

function applyRec(codes) {
  const selected = new Set(codes);
  document.querySelectorAll("#hand .card").forEach((node) => {
    node.classList.toggle("selected", selected.has(node.dataset.code));
  });
  updateSelectedCount();
  logCardDiagnostics("recommendation applied");
}

function renderRecommendations() {
  const container = el("recommendations");
  if (!container) return;
  container.innerHTML = "";

  const recs = state.recommendations;
  if (!recs.length) {
    const empty = document.createElement("span");
    empty.className = "recommend-empty";
    empty.textContent = isGameOver() ? "本局已结束" : "暂无可压过上家的推荐";
    container.appendChild(empty);
    return;
  }

  const selected = selectedCardsSet();
  recs.forEach((rec) => {
    const btn = document.createElement("button");
    btn.type = "button";
    btn.className = "recommend-chip";
    btn.textContent = recLabel(rec);
    if (rec.codes.length === selected.size && rec.codes.every((code) => selected.has(code))) {
      btn.classList.add("active");
    }
    btn.addEventListener("click", () => {
      applyRec(rec.codes);
      renderRecommendations();
    });
    container.appendChild(btn);
  });
}

function refreshRecommendations() {
  state.recommendations = buildRecommendations();
  renderRecommendations();
}

function updateTurnBanner() {
  const banner = el("turnBanner");
  if (!banner) return;

  let text = "正在连接服务器...";
  let tone = "pending";

  if (!state.connected) {
    text = state.reconnectAttempts > 0 ? "连接中断，正在重连..." : "正在连接服务器...";
    tone = "pending";
  } else if (!state.roomId) {
    text = "已连接，请从房间列表直接加入，或创建新房间";
    tone = "ready";
  } else if (isGameOver()) {
    text = `对局结束，胜者：${nameById(state.gameOverWinnerId)}`;
    tone = "ready";
  } else if (!state.roomStarted) {
    text = `房间 ${state.roomId} 等待玩家加入（${state.currentRoomPlayerCount}/3）`;
    tone = "ready";
  } else if (state.turn === state.userId) {
    text = "轮到你出牌";
    tone = "active";
  } else if (state.turn) {
    text = `等待 ${nameById(state.turn)} 出牌`;
    tone = "wait";
  }

  banner.textContent = text;
  banner.dataset.tone = tone;
}

function updateActionState() {
  const selected = state.selectedRoomId
    ? state.roomList.find((room) => room.room_id === state.selectedRoomId)
    : null;
  const canJoin = state.connected && selected && selected.can_join && state.selectedRoomId !== state.roomId;
  const myTurn = state.connected
    && state.roomStarted
    && !isGameOver()
    && state.turn === state.userId;

  if (el("reconnectBtn")) el("reconnectBtn").disabled = !state.serverUrl;
  if (el("createRoomBtn")) el("createRoomBtn").disabled = !state.connected;
  if (el("joinRoomBtn")) el("joinRoomBtn").disabled = !canJoin;
  if (el("refreshRoomsBtn")) el("refreshRoomsBtn").disabled = !state.connected;
  if (el("playBtn")) el("playBtn").disabled = !myTurn;
  if (el("passBtn")) el("passBtn").disabled = !myTurn;
  if (el("clearBtn")) el("clearBtn").disabled = state.hand.length === 0 || isGameOver();
  if (el("restartGameBtn")) {
    el("restartGameBtn").disabled = !isGameOver() || !state.connected || !state.roomId;
  }
}

export function formatPlay(play) {
  if (!play) return "暂无出牌";
  const kind = KIND_LABEL[play.kind] || play.kind;
  const rank = rankToLabel(play.main_rank);
  const size = play.size ? ` · ${play.size} 张` : "";
  return `${kind} · 主牌 ${rank}${size}`;
}

export function renderPlayers(container, players, turnId) {
  if (!container) return;
  container.innerHTML = "";
  players.forEach((player) => {
    const row = document.createElement("div");
    row.className = `player${player.id === turnId ? " active" : ""}`;
    row.innerHTML = `<span>${playerLabel(player)}（${player.is_landlord ? "地主" : "农民"}）</span><span>${player.hand_count}张</span>`;
    container.appendChild(row);
  });
}

export function renderHand(container, hand) {
  if (!container) return;
  container.innerHTML = "";

  if (!hand || hand.length === 0) {
    const empty = document.createElement("div");
    empty.className = "hand-empty";
    empty.textContent = "暂无手牌，等待发牌...";
    container.appendChild(empty);
    updateSelectedCount();
    syncHandLayout(container);
    return;
  }

  hand.forEach((code, index) => {
    const card = document.createElement("button");
    card.type = "button";
    card.className = "card";
    card.dataset.code = code;
    card.style.setProperty("--stack-order", String(index + 1));

    const parsed = parseCard(code);
    const label = parsed ? parsed.label : code;
    card.dataset.tone = parsed ? parsed.tone : "unknown";
    card.setAttribute("aria-label", label);
    card.title = label;

    const image = document.createElement("img");
    image.className = "card-image";
    image.src = cardImage(code);
    image.alt = label;
    image.loading = "lazy";
    image.addEventListener("error", () => {
      image.src = cardBackImageUrl();
    });

    const caption = document.createElement("span");
    caption.className = "card-label";
    caption.textContent = label;

    card.appendChild(image);
    card.appendChild(caption);

    card.addEventListener("click", () => {
      card.classList.toggle("selected");
      updateSelectedCount();
      renderRecommendations();
      logCardDiagnostics(`click ${code}`);
    });

    container.appendChild(card);
  });

  updateSelectedCount();
  syncHandLayout(container);
  logCardDiagnostics("render hand");
}

export function selectedCards() {
  return Array.from(document.querySelectorAll("#hand .card.selected")).map((node) => node.dataset.code);
}

export function clearSelection() {
  document.querySelectorAll("#hand .card.selected").forEach((node) => node.classList.remove("selected"));
  updateSelectedCount();
  renderRecommendations();
  logCardDiagnostics("clear selection");
}

export function applyRoomState(snapshot) {
  state.roomId = snapshot.room_id;
  state.selectedRoomId = snapshot.room_id;
  state.players = snapshot.players || [];
  state.hand = sortHand(snapshot.your_hand || []);
  state.turn = snapshot.turn;
  state.lastPlay = snapshot.last_play;
  state.lastPlayer = snapshot.last_player;
  state.currentRoomPlayerCount = state.players.length;
  state.roomStarted = true;
  state.players.forEach((player) => saveName(player.id, player.name));
  const someoneOut = state.players.some((player) => player.hand_count === 0);
  if (!someoneOut && isGameOver()) {
    // If we are in an active round snapshot, stale game-over UI must be cleared.
    setGameOver(null, "active-room-state");
  }

  setText("roomId", snapshot.room_id || "-");
  setText("myRole", "-");
  setText("myHandCount", `${state.hand.length} 张`);
  setText("lastPlay", formatPlay(snapshot.last_play));
  setText("lastPlayer", describeLastPlayer(snapshot.last_player));
  syncRoomInput();

  const seats = buildSeats(state.players);
  renderOpponentSeat(el("leftSeat"), seats.left, state.turn, "左侧");
  renderOpponentSeat(el("rightSeat"), seats.right, state.turn, "右侧");

  if (seats.self) {
    setText("myRole", seats.self.is_landlord ? "地主" : "农民");
  }

  renderPlayers(el("players"), state.players, state.turn);
  renderHand(el("hand"), state.hand);
  renderRoomList(el("roomList"), state.roomList);
  refreshRecommendations();
  updateTurnBanner();
  updateActionState();
}

export function handleServerMessage(msg) {
  switch (msg.type) {
    case "Welcome":
      state.userId = msg.data.user_id;
      state.userName = msg.data.user_name || `Player_${String(msg.data.user_id).slice(-4)}`;
      saveName(state.userId, state.userName);
      setText("userId", state.userName);
      logMessage(`欢迎你，${state.userName}`);
      updateTurnBanner();
      break;

    case "RoomsList":
      state.roomList = msg.data.rooms || [];
      if (state.selectedRoomId && !state.roomList.some((room) => room.room_id === state.selectedRoomId)) {
        state.selectedRoomId = state.roomId;
      }
      if (!state.selectedRoomId && state.roomList.length > 0) {
        state.selectedRoomId = state.roomList[0].room_id;
      }
      renderRoomList(el("roomList"), state.roomList);
      syncRoomInput();
      updateActionState();
      updateTurnBanner();
      break;

    case "RoomCreated":
      logMessage(`房间创建成功：${msg.data.room_id}`);
      requestRoomList();
      break;

    case "Joined":
      state.roomId = msg.data.room_id;
      state.selectedRoomId = msg.data.room_id;
      state.currentRoomPlayerCount = msg.data.player_count || 0;
      state.roomStarted = Boolean(msg.data.started);
      setGameOver(null, "joined");
      saveName(msg.data.you, msg.data.you_name);
      setText("roomId", state.roomId);
      syncRoomInput();
      logMessage(`已加入房间：${state.roomId}`);
      updateTurnBanner();
      updateActionState();
      requestRoomList();
      break;

    case "RoomState":
      applyRoomState(msg.data);
      break;

    case "PlayRejected":
      logMessage(`出牌失败：${msg.data.reason}`);
      break;

    case "GameOver":
      if (!msg?.data?.room_id || !state.roomId || msg.data.room_id !== state.roomId || !state.roomStarted) {
        logMessage(`忽略无效结束事件：${msg?.data?.room_id || "unknown-room"}`);
        break;
      }
      setGameOver(msg.data.winner_id, "server-message");
      logMessage(`对局结束，赢家：${nameById(msg.data.winner_id)}`);
      updateTurnBanner();
      updateActionState();
      refreshRecommendations();
      break;

    case "RoomInterrupted":
      if (!msg?.data?.room_id || !state.roomId || msg.data.room_id !== state.roomId) {
        break;
      }
      state.roomStarted = false;
      state.currentRoomPlayerCount = Number(msg.data.player_count) || 0;
      state.turn = null;
      state.lastPlay = null;
      state.lastPlayer = null;
      state.hand = [];
      state.recommendations = [];
      setGameOver(null, "room-interrupted");
      setText("myRole", "-");
      setText("myHandCount", "0 张");
      setText("lastPlay", "对局已结束（玩家离开）");
      setText("lastPlayer", "等待玩家加入后自动开局");
      renderHand(el("hand"), []);
      renderRoomList(el("roomList"), state.roomList);
      refreshRecommendations();
      clearSelection();
      logMessage(`玩家 ${nameById(msg.data.leaver_id)} 离开房间，对局已结束，等待补齐玩家重开`);
      updateTurnBanner();
      updateActionState();
      requestRoomList();
      break;

    case "GameRestarted":
      setGameOver(null, "restart");
      clearSelection();
      logMessage(`房间 ${msg.data.room_id} 已开始新一局`);
      updateTurnBanner();
      updateActionState();
      break;

    case "Error":
      logMessage(`服务器错误：${msg.data.message}`);
      break;

    case "Pong":
      break;

    default:
      logMessage("收到未知消息");
  }
}

function bindClick(id, handler) {
  const node = el(id);
  if (node) node.addEventListener("click", handler);
}

export function init() {
  if (initialized) return;
  initialized = true;

  state.serverUrl = readUrl();
  if (el("serverUrl")) el("serverUrl").value = state.serverUrl;
  if (el("roomIdInput")) el("roomIdInput").readOnly = true;
  if (el("gameOverModal")) el("gameOverModal").hidden = true;
  if (typeof window !== "undefined") {
    window.addEventListener("resize", () => syncHandLayout());
  }

  bindClick("reconnectBtn", () => connect((el("serverUrl")?.value || DEFAULT_WS_URL).trim()));
  bindClick("createRoomBtn", () => {
    if (sendMessage({ type: "CreateRoom" })) logMessage("正在创建房间...");
  });
  bindClick("joinRoomBtn", () => {
    if (state.selectedRoomId) joinRoom(state.selectedRoomId);
    else logMessage("请先在房间列表中选择一个房间");
  });
  bindClick("refreshRoomsBtn", requestRoomList);
  bindClick("playBtn", () => {
    const cards = selectedCards();
    if (!cards.length) {
      logMessage("请先选择要出的牌");
      return;
    }
    if (sendMessage({ type: "Play", data: { cards } })) clearSelection();
  });
  bindClick("passBtn", () => sendMessage({ type: "Pass" }));
  bindClick("clearBtn", clearSelection);
  bindClick("restartGameBtn", () => {
    if (!state.connected || !state.roomId) {
      logMessage("当前不在可重开的房间中");
      return;
    }
    if (sendMessage({ type: "RestartGame" })) {
      logMessage("已请求再来一局...");
    }
  });

  resetRoom();
  setStatus("自动连接中...", "pending");
  updateTurnBanner();
  updateActionState();
  connect(state.serverUrl);
}

const IS_TEST_ENV = typeof navigator !== "undefined" && /jsdom/i.test(navigator.userAgent || "");
if (typeof window !== "undefined" && !IS_TEST_ENV) {
  window.addEventListener("DOMContentLoaded", () => init());
}

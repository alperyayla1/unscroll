const { invoke } = window.__TAURI__.core;
const appWindow = window.__TAURI__.window.getCurrentWindow();

const $ = (id) => document.getElementById(id);
const PRESETS = [30, 60, 120, 240, 480, 1440, 10080]; // minutes
const HOLD_MS = 1500;

let view = null;
let viewAt = 0; // performance.now() when `view` arrived
let selected = null; // minutes, or "custom"
let extending = false;
let shownLoadNotice = false;

// ---------------------------------------------------------------- helpers

function toast(msg) {
  const el = $("toast");
  el.textContent = msg;
  el.hidden = false;
  clearTimeout(toast.t);
  toast.t = setTimeout(() => (el.hidden = true), 3200);
}

function errText(e) {
  const code = String(e);
  if (code === "locked") return t("err_locked");
  if (code === "bad_duration") return t("err_bad_duration");
  return t("err_generic", { e: code });
}

async function call(cmd, args) {
  try {
    const v = await invoke(cmd, args);
    setView(v);
    return true;
  } catch (e) {
    toast(errText(e));
    refresh();
    return false;
  }
}

function setView(v) {
  view = v;
  viewAt = performance.now();
  if (!shownLoadNotice && (v.load === "restored" || v.load === "reset")) {
    shownLoadNotice = true;
    toast(t(v.load));
  }
  render();
}

async function refresh() {
  try {
    setView(await invoke("get_view"));
  } catch (e) {
    toast(errText(e));
  }
}

function remainingNow() {
  if (!view || !view.locked) return 0;
  return Math.max(0, view.remainingMs - (performance.now() - viewAt));
}

function hms(ms) {
  const s = Math.ceil(ms / 1000);
  const d = Math.floor(s / 86400);
  const h = Math.floor((s % 86400) / 3600);
  const m = Math.floor((s % 3600) / 60);
  const sec = s % 60;
  const pad = (n) => String(n).padStart(2, "0");
  return (d ? `${d}${t("unit_d")} ` : "") + `${pad(h)}:${pad(m)}:${pad(sec)}`;
}

function selectedMinutes() {
  if (selected === "custom") {
    const d = parseInt($("in-days").value || "0", 10) || 0;
    const h = parseInt($("in-hours").value || "0", 10) || 0;
    const m = parseInt($("in-mins").value || "0", 10) || 0;
    return Math.max(0, d * 1440 + h * 60 + m);
  }
  return selected || 0;
}

// ---------------------------------------------------------------- static text

function applyStaticText() {
  document.documentElement.lang = LANG;
  $("levels-legend").innerHTML = t("levels_legend_html");
  document.querySelectorAll("[data-i18n]").forEach((el) => (el.textContent = t(el.dataset.i18n)));
  const chips = $("chips");
  chips.innerHTML = "";
  for (const m of PRESETS) {
    const b = document.createElement("button");
    b.className = "chip";
    b.textContent = fmtDuration(m);
    b.dataset.min = m;
    b.onclick = () => select(m);
    chips.appendChild(b);
  }
  const c = document.createElement("button");
  c.className = "chip";
  c.textContent = t("custom");
  c.dataset.min = "custom";
  c.onclick = () => select("custom");
  chips.appendChild(c);
}

function select(m) {
  selected = selected === m ? null : m;
  $("custom").hidden = selected !== "custom";
  if (selected === "custom") $("in-hours").focus();
  renderPicker();
}

// ---------------------------------------------------------------- render

const PUZZLE = '<svg viewBox="0 0 24 24"><path d="M10 3h4v3a2 2 0 1 0 4 0h0v4h-3a2 2 0 1 0 0 4h3v4h-4v-3a2 2 0 1 0-4 0v3H6v-4h3a2 2 0 1 0 0-4H6V6h4z"/></svg>';

function shortsActive() {
  return view.locked || view.shortsOn;
}
function sitesActive() {
  return view.locked || view.sitesOn;
}

function render() {
  if (!view) return;
  const ext = view.extension;
  const anyShorts = view.platforms.some((p) => p.level === "shorts" || (p.allShorts && p.level === "site"));
  const partial = shortsActive() && anyShorts && !ext.connected;

  const pill = $("pill");
  let st = view.locked ? "locked" : !view.active ? "off" : partial ? "partial" : "on";
  pill.className = "pill " + st;
  pill.textContent = t("status_" + st);

  // Shorts card
  const ts = $("toggle-shorts");
  ts.classList.toggle("on", shortsActive());
  ts.setAttribute("aria-checked", String(shortsActive()));
  ts.disabled = view.locked;
  $("card-shorts").classList.toggle("dim", !shortsActive());
  $("ext-status").className = "ext-status " + (ext.connected ? "ok" : "bad");
  $("ext-dot").className = "dot " + (ext.connected ? "ok" : "bad");
  $("ext-title").textContent = ext.connected ? t("ext_ok_title") : t("ext_off_title");
  $("ext-sub").textContent = !shortsActive()
    ? t("ext_paused_sub")
    : ext.connected
    ? t("ext_ok_sub", { b: ext.browser || "browser" })
    : t("ext_off_sub");
  $("ext-setup").hidden = ext.connected;

  // Sites card
  const tsi = $("toggle-sites");
  tsi.classList.toggle("on", sitesActive());
  tsi.setAttribute("aria-checked", String(sitesActive()));
  tsi.disabled = view.locked;
  $("card-sites").classList.toggle("dim", !sitesActive());
  const chips = $("site-chips");
  chips.innerHTML = "";
  let anySite = false;
  for (const p of view.platforms) {
    if (p.allShorts) continue;
    const sel = p.level === "site";
    anySite = anySite || sel;
    const b = document.createElement("button");
    b.className = "site-chip" + (sel ? " sel" : "");
    b.textContent = p.name;
    b.disabled = sel && p.floor === "site";
    b.onclick = async () => {
      const next = sel ? (p.supportsShorts ? "shorts" : "open") : "site";
      const ok = await call("set_level", { id: p.id, level: next });
      if (ok && next === "site" && !view.sitesOn && !view.locked) call("set_sites", { on: true });
    };
    chips.appendChild(b);
  }
  $("sites-empty").hidden = anySite;

  $("lock-running").hidden = !view.locked;
  $("lock-picker").hidden = view.locked && !extending;
  if (!view.locked) extending = false;
  renderPicker();
  tickCountdown();
  renderSettings();
  maybeFirstRunSetup();
}

function maybeFirstRunSetup() {
  if (view.extension.connected) return;
  let seen = false;
  try {
    seen = localStorage.getItem("unscroll.setupShown") === "1";
    localStorage.setItem("unscroll.setupShown", "1");
  } catch (_) {
    seen = true;
  }
  if (!seen) $("setup").hidden = false;
}

function renderPicker() {
  document.querySelectorAll(".chip").forEach((c) => {
    const val = c.dataset.min === "custom" ? "custom" : Number(c.dataset.min);
    c.classList.toggle("sel", val === selected);
  });
  const mins = selectedMinutes();
  const btn = $("btn-lock");
  btn.disabled = mins < 1;
  btn.textContent = mins < 1 ? t("pick_time") : extending ? `${t("extend")} · ${fmtDuration(mins)}` : t("start_lock", { t: fmtDuration(mins) });
}

function tickCountdown() {
  if (!view || !view.locked) return;
  const rem = remainingNow();
  $("countdown").textContent = hms(rem);
  const pct = view.totalMs ? (1 - rem / view.totalMs) * 100 : 0;
  $("bar").style.width = `${Math.min(100, Math.max(0, pct))}%`;
  const end = new Date(Date.now() + rem);
  const sameDay = end.toDateString() === new Date().toDateString();
  const when = end.toLocaleString(LANG, sameDay ? { hour: "2-digit", minute: "2-digit" } : { weekday: "short", day: "numeric", month: "short", hour: "2-digit", minute: "2-digit" });
  $("ends-at").textContent = t("ends_at", { when });
  if (rem <= 0) refresh();
}

const LEVELS = ["open", "shorts", "site"];
const RANK = { open: 0, shorts: 1, site: 2 };

function renderSettings() {
  const list = $("platforms");
  list.innerHTML = "";
  for (const p of view.platforms) {
    const row = document.createElement("div");
    row.className = "prow";
    const name = document.createElement("div");
    name.className = "pname";
    name.textContent = p.name;
    if (!p.supportsShorts || p.allShorts) {
      const hint = document.createElement("small");
      hint.textContent = p.allShorts ? t("all_shorts") : t("no_shorts_feed");
      name.appendChild(hint);
    }
    const seg = document.createElement("div");
    seg.className = "seg";
    for (const lvl of LEVELS) {
      const b = document.createElement("button");
      b.textContent = t("lvl_" + lvl);
      if (lvl === "shorts" && p.supportsShorts && !p.allShorts) {
        b.insertAdjacentHTML("beforeend", PUZZLE);
        b.title = t("ext_icon_title");
      }
      const unsupported = lvl === "shorts" && (!p.supportsShorts || p.allShorts);
      const belowFloor = RANK[lvl] < RANK[p.floor];
      b.disabled = unsupported || belowFloor;
      if (belowFloor) b.classList.add("floor");
      if (unsupported) b.classList.add("na");
      b.classList.toggle("sel", p.level === lvl);
      if (p.level === lvl && lvl !== "open") b.classList.add(lvl);
      b.onclick = () => p.level !== lvl && call("set_level", { id: p.id, level: lvl });
      seg.appendChild(b);
    }
    row.append(name, seg);
    list.appendChild(row);
  }

  const hard = $("hardening");
  hard.innerHTML = "";
  for (const [key, title, sub] of [
    ["no_incognito", "hard_incognito", "hard_incognito_sub"],
    ["no_guest", "hard_guest", "hard_guest_sub"],
    ["no_extension_page", "hard_ext", "hard_ext_sub"],
  ]) {
    const row = document.createElement("div");
    row.className = "hrow";
    const txt = document.createElement("div");
    txt.innerHTML = `<div class="htitle"></div><small></small>`;
    txt.querySelector(".htitle").textContent = t(title);
    txt.querySelector("small").textContent = t(sub);
    const tg = document.createElement("button");
    const on = view.hardening[key];
    tg.className = "toggle" + (on ? " on" : "");
    tg.setAttribute("role", "switch");
    tg.setAttribute("aria-checked", String(on));
    tg.innerHTML = "<span></span>";
    tg.disabled = view.hardFloor[key];
    tg.onclick = () => call("set_hard", { key, on: !on });
    row.append(txt, tg);
    hard.appendChild(row);
  }
}

// ---------------------------------------------------------------- confirm + hold

function openConfirm() {
  const mins = selectedMinutes();
  if (mins < 1) return;
  $("confirm-title").textContent = t("confirm_title", { t: fmtDuration(mins) });
  const ul = $("confirm-summary");
  ul.innerHTML = "";
  const add = (text) => {
    const li = document.createElement("li");
    li.textContent = text;
    ul.appendChild(li);
  };
  const shorts = view.platforms.filter((p) => p.level === "shorts").map((p) => p.name);
  const tiktokLike = view.platforms.filter((p) => p.allShorts && p.level === "site").map((p) => p.name);
  const siteOnly = view.platforms.filter((p) => !p.allShorts && p.level === "site").map((p) => p.name);
  // A lock with nothing selected uses the defaults (see State::start_lock).
  if (!shorts.length && !tiktokLike.length && !siteOnly.length) shorts.push("YouTube", "Instagram", "Facebook", "Snapchat"), tiktokLike.push("TikTok");
  const shortsAll = shorts.concat(tiktokLike);
  if (shortsAll.length) add(t("confirm_shorts_ext", { list: shortsAll.join(", ") }));
  if (siteOnly.length) add(t("confirm_site_noext", { list: siteOnly.join(", ") }));
  if (shortsAll.length && !view.extension.connected) add(t("confirm_no_ext"));
  const h = view.hardening;
  if (h.no_incognito || h.no_guest || h.no_extension_page) add(t("confirm_hard"));
  if (view.locked) {
    const total = Math.round(remainingNow() / 60000) + mins;
    add(t("confirm_extend", { t: fmtDuration(mins), total: fmtDuration(total) }));
  }
  resetHold();
  $("confirm").hidden = false;
}

let holdTimer = null;
function resetHold() {
  clearTimeout(holdTimer);
  holdTimer = null;
  $("hold").classList.remove("holding");
}

function startHold(e) {
  e.preventDefault();
  if (holdTimer) return;
  $("hold").classList.add("holding");
  holdTimer = setTimeout(async () => {
    holdTimer = null;
    const mins = selectedMinutes();
    $("confirm").hidden = true;
    $("hold").classList.remove("holding");
    if (await call("start_lock", { minutes: mins })) {
      selected = null;
      extending = false;
      $("custom").hidden = true;
      ["in-days", "in-hours", "in-mins"].forEach((id) => ($(id).value = ""));
      render();
    }
  }, HOLD_MS);
}

// ---------------------------------------------------------------- wiring

function wire() {
  $("btn-min").onclick = () => appWindow.minimize();
  $("btn-close").onclick = () => appWindow.close();

  $("toggle-shorts").onclick = () => {
    if (!view || view.locked) return;
    call("set_shorts", { on: !view.shortsOn });
  };
  $("toggle-sites").onclick = () => {
    if (!view || view.locked) return;
    call("set_sites", { on: !view.sitesOn });
  };

  ["in-days", "in-hours", "in-mins"].forEach((id) => ($(id).oninput = renderPicker));
  $("btn-lock").onclick = openConfirm;
  $("btn-extend").onclick = () => {
    extending = !extending;
    render();
  };

  const hold = $("hold");
  hold.addEventListener("pointerdown", startHold);
  ["pointerup", "pointerleave", "pointercancel"].forEach((ev) => hold.addEventListener(ev, resetHold));
  $("confirm-cancel").onclick = () => {
    resetHold();
    $("confirm").hidden = true;
  };

  $("open-settings").onclick = () => {
    $("view-main").hidden = true;
    $("view-settings").hidden = false;
  };
  $("close-settings").onclick = () => {
    $("view-settings").hidden = true;
    $("view-main").hidden = false;
  };

  $("ext-setup").onclick = () => ($("setup").hidden = false);
  $("setup-close").onclick = () => {
    $("setup").hidden = true;
    refresh();
  };
  $("open-folder").onclick = () => invoke("open_extension_folder");
  $("copy-url").onclick = async () => {
    try {
      await navigator.clipboard.writeText($("ext-url").textContent);
      $("copy-url").textContent = t("copied");
      setTimeout(() => ($("copy-url").textContent = t("copy")), 1500);
    } catch (_) {}
  };

  document.addEventListener("keydown", (e) => {
    if (e.key === "Escape") {
      resetHold();
      $("confirm").hidden = true;
      $("setup").hidden = true;
    }
  });
  document.addEventListener("contextmenu", (e) => e.preventDefault());
}

applyStaticText();
wire();
refresh();
setInterval(tickCountdown, 1000);
setInterval(refresh, 15000);

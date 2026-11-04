import { NATIVE_HOST, PLATFORMS, DEFAULT_CONFIG, blockedPatterns, match, blockedPath } from "./rules.js";

const POLL_MINUTES = 0.5;

function browserName() {
  const brands = (navigator.userAgentData && navigator.userAgentData.brands) || [];
  const names = brands.map((b) => b.brand);
  if (names.some((n) => /Edge/i.test(n))) return "Edge";
  if (names.some((n) => /Brave/i.test(n))) return "Brave";
  if (names.some((n) => /Opera/i.test(n)) || /OPR\//.test(navigator.userAgent)) return "Opera";
  return "Chrome";
}

async function getCached() {
  const { config } = await chrome.storage.local.get("config");
  return config || null;
}

/** Asks the desktop app for the current config. Falls back to the cache, then to the defaults. */
async function refresh() {
  let cfg;
  try {
    const r = await chrome.runtime.sendNativeMessage(NATIVE_HOST, { cmd: "get", browser: browserName() });
    if (!r || !r.ok) throw new Error("bad reply");
    cfg = { active: r.active, locked: r.locked, remainingMs: r.remainingMs, levels: r.levels, connected: true, at: Date.now() };
  } catch (_) {
    const cached = await getCached();
    cfg = { ...(cached || DEFAULT_CONFIG), connected: false, at: cached ? cached.at : Date.now() };
  }
  await chrome.storage.local.set({ config: cfg });
  await applyRules(cfg);
  await sweepTabs(cfg);
  broadcast(cfg);
  return cfg;
}

/** Network-level block: redirects page loads before anything is fetched. */
async function applyRules(cfg) {
  const rules = [];
  let id = 1;
  for (const pid of Object.keys(PLATFORMS)) {
    const kind = cfg.active ? cfg.levels[pid] : "open";
    for (const regexFilter of blockedPatterns(cfg, pid)) {
      rules.push({
        id: id++,
        priority: 1,
        action: { type: "redirect", redirect: { extensionPath: blockedPath(pid, kind) } },
        condition: { regexFilter, resourceTypes: ["main_frame", "sub_frame"] },
      });
    }
  }
  const existing = await chrome.declarativeNetRequest.getDynamicRules();
  const same =
    existing.length === rules.length &&
    existing.every((r, i) => JSON.stringify(r.condition.regexFilter) === JSON.stringify(rules[i].condition.regexFilter) && r.action.redirect.extensionPath === rules[i].action.redirect.extensionPath);
  if (same) return;
  await chrome.declarativeNetRequest.updateDynamicRules({
    removeRuleIds: existing.map((r) => r.id),
    addRules: rules,
  });
}

function redirectTab(tabId, url, cfg) {
  const m = match(cfg, url);
  if (!m) return;
  chrome.tabs.update(tabId, { url: chrome.runtime.getURL(blockedPath(m.id, m.kind)) }).catch(() => {});
}

/** Tabs that were already open on something that is now blocked. */
async function sweepTabs(cfg) {
  const tabs = await chrome.tabs.query({});
  for (const tab of tabs) if (tab.url) redirectTab(tab.id, tab.url, cfg);
}

function broadcast(cfg) {
  chrome.tabs.query({}).then((tabs) => {
    for (const tab of tabs) chrome.tabs.sendMessage(tab.id, { type: "config", config: cfg }).catch(() => {});
  });
}

// Single-page apps (YouTube, Instagram, Facebook) change the URL without a page
// load, which network rules don't see. Catch those here.
async function onSpaNavigation(details) {
  if (details.frameId !== 0) return;
  const cfg = (await getCached()) || DEFAULT_CONFIG;
  redirectTab(details.tabId, details.url, cfg);
}
chrome.webNavigation.onHistoryStateUpdated.addListener(onSpaNavigation);
chrome.webNavigation.onReferenceFragmentUpdated.addListener(onSpaNavigation);

// Content scripts ask for their platform's level.
chrome.runtime.onMessage.addListener((msg, _sender, reply) => {
  if (msg && msg.type === "get") {
    getCached().then((cfg) => reply({ config: cfg || DEFAULT_CONFIG }));
    return true;
  }
  if (msg && msg.type === "refresh") {
    refresh().then((cfg) => reply({ config: cfg }));
    return true;
  }
});

chrome.alarms.onAlarm.addListener((a) => a.name === "poll" && refresh());
function start() {
  chrome.alarms.create("poll", { periodInMinutes: POLL_MINUTES });
  refresh();
}
chrome.runtime.onInstalled.addListener(start);
chrome.runtime.onStartup.addListener(start);
// Service workers can be woken for other reasons; make sure polling is set up.
chrome.alarms.get("poll").then((a) => !a && start());

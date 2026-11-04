import { PLATFORMS, levelOf } from "./rules.js";
import { t, fmtLeft } from "./text.js";

function render(cfg) {
  const pill = document.getElementById("pill");
  if (cfg.locked && cfg.remainingMs > 0) {
    pill.className = "pill locked";
    pill.textContent = `${t("st_locked")} · ${fmtLeft(cfg.remainingMs - (Date.now() - (cfg.at || Date.now())))}`;
  } else {
    pill.className = "pill " + (cfg.active ? "on" : "off");
    pill.textContent = cfg.active ? t("st_on") : t("st_off");
  }

  const rows = document.getElementById("rows");
  rows.innerHTML = "";
  for (const [id, p] of Object.entries(PLATFORMS)) {
    const lvl = levelOf(cfg, id);
    const r = document.createElement("div");
    r.className = "r";
    const a = document.createElement("span");
    a.textContent = p.name;
    const b = document.createElement("span");
    b.textContent = t("lv_" + lvl);
    b.className = lvl;
    r.append(a, b);
    rows.appendChild(r);
  }

  document.getElementById("dot").className = "dot " + (cfg.connected ? "ok" : "bad");
  document.getElementById("conn").textContent = cfg.connected ? t("connected") : t("not_connected");
  document.getElementById("note").textContent = t("controls_in_app");
}

chrome.storage.local.get("config").then(({ config }) => config && render(config));
chrome.runtime.sendMessage({ type: "refresh" }).then((r) => r && r.config && render(r.config)).catch(() => {});

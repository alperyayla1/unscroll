import { PLATFORMS } from "./rules.js";
import { t, fmtLeft } from "./text.js";

const q = new URLSearchParams(location.search);
const id = q.get("p");
const kind = q.get("k");
const p = PLATFORMS[id] || { name: "This site", home: null };

const isShorts = kind === "shorts" && PLATFORMS[id] && PLATFORMS[id].shorts;
document.getElementById("title").textContent = isShorts ? t("shorts_title") : t("site_title", { p: p.name });
document.getElementById("sub").textContent = isShorts ? t("shorts_sub", { p: p.name }) : t("site_sub");

const home = document.getElementById("home");
if (isShorts && p.home) {
  home.hidden = false;
  home.textContent = t("go_home", { p: p.name });
  home.onclick = () => location.replace(p.home);
}

const close = document.getElementById("close");
close.textContent = t("close");
close.onclick = async () => {
  const tab = await chrome.tabs.getCurrent();
  if (tab) chrome.tabs.remove(tab.id);
  else window.close();
};

async function showLock() {
  const { config } = await chrome.storage.local.get("config");
  const el = document.getElementById("lock");
  if (config && config.locked && config.remainingMs > 0) {
    const left = config.remainingMs - (Date.now() - (config.at || Date.now()));
    el.hidden = left <= 0;
    el.textContent = t("locked_for", { t: fmtLeft(left) });
  } else {
    el.hidden = true;
  }
}
showLock();
setInterval(showLock, 30000);

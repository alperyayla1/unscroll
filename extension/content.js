// Hides short-video shelves, tabs and links on sites where the rest stays usable.
// Navigation to short-video pages is handled by the background script.
(() => {
  const host = location.hostname;
  const P = /(^|\.)youtube\.com$/.test(host)
    ? "youtube"
    : /(^|\.)instagram\.com$/.test(host)
    ? "instagram"
    : /(^|\.)facebook\.com$/.test(host)
    ? "facebook"
    : /(^|\.)snapchat\.com$/.test(host)
    ? "snapchat"
    : null;
  if (!P) return;

  const shortsLink = 'a[href^="/shorts/"], a[href^="https://www.youtube.com/shorts/"]';
  const CSS = {
    youtube: `
      ytd-reel-shelf-renderer, ytm-reel-shelf-renderer,
      ytd-rich-shelf-renderer[is-shorts], ytd-rich-section-renderer:has(ytd-rich-shelf-renderer[is-shorts]),
      ytd-rich-section-renderer:has(ytm-shorts-lockup-view-model), ytd-rich-section-renderer:has(ytm-shorts-lockup-view-model-v2),
      grid-shelf-view-model:has(ytm-shorts-lockup-view-model), grid-shelf-view-model:has(ytm-shorts-lockup-view-model-v2),
      ytd-item-section-renderer:has(> #contents > grid-shelf-view-model ytm-shorts-lockup-view-model-v2),
      ytm-shorts-lockup-view-model, ytm-shorts-lockup-view-model-v2,
      ytd-video-renderer:has(${shortsLink}), ytd-grid-video-renderer:has(${shortsLink}),
      ytd-rich-item-renderer:has(${shortsLink}), ytd-compact-video-renderer:has(${shortsLink}),
      yt-lockup-view-model:has(${shortsLink}), ytm-video-with-context-renderer:has(${shortsLink}),
      ytd-notification-renderer:has(${shortsLink}),
      ytd-guide-entry-renderer:has(a[title="Shorts"]), ytd-mini-guide-entry-renderer[aria-label="Shorts"],
      ytd-mini-guide-entry-renderer:has(a[title="Shorts"]),
      yt-tab-shape[tab-title="Shorts"], ytm-pivot-bar-item-renderer:has(.pivot-shorts)
      { display: none !important; }`,
    instagram: `
      a[href="/reels/"], a[href^="/reels/"], a[href$="/reels/"], a[href="/explore/"], a[href^="/explore/"],
      article:has(a[href*="/reel/"])
      { display: none !important; }`,
    facebook: `
      a[href*="/reel/"], a[href*="/reels/"], a[href^="/watch"], a[href^="https://www.facebook.com/watch"],
      [role="article"]:has(a[href*="/reel/"]), [data-pagelet^="FeedUnit"]:has(a[href*="/reel/"]),
      [aria-label="Reels"], [aria-label="Reels and short videos"]
      { display: none !important; }`,
    snapchat: `a[href*="/spotlight"] { display: none !important; }`,
  };

  let style = null;
  let on = false;

  function setOn(v) {
    on = v;
    if (v && !style) {
      style = document.createElement("style");
      style.id = "unshorts-hide";
      style.textContent = CSS[P];
      (document.head || document.documentElement).appendChild(style);
    } else if (!v && style) {
      style.remove();
      style = null;
    }
    if (v) sweep();
  }

  // Things CSS alone can't target reliably.
  function sweep() {
    if (!on) return;
    if (P === "youtube") {
      document.querySelectorAll("yt-chip-cloud-chip-renderer, chip-shape").forEach((c) => {
        if (c.textContent.trim() === "Shorts") c.style.setProperty("display", "none", "important");
      });
    }
    if (P === "facebook") {
      document.querySelectorAll('a[href*="/reel/"]').forEach((a) => {
        const unit = a.closest('[role="article"], [data-pagelet^="FeedUnit"]');
        if (unit) unit.style.setProperty("display", "none", "important");
      });
    }
  }

  let queued = false;
  new MutationObserver(() => {
    if (!on || queued) return;
    queued = true;
    requestAnimationFrame(() => {
      queued = false;
      sweep();
    });
  }).observe(document.documentElement, { childList: true, subtree: true });

  const apply = (cfg) => setOn(!!cfg && cfg.active && cfg.levels && cfg.levels[P] === "shorts");

  // Fail closed until the background answers.
  setOn(true);
  chrome.runtime.sendMessage({ type: "get" }).then((r) => r && apply(r.config)).catch(() => {});
  chrome.runtime.onMessage.addListener((msg) => {
    if (msg && msg.type === "config") apply(msg.config);
  });
})();

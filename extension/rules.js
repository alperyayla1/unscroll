// Platform rules. Keep in sync with core/src/platforms.rs.
export const NATIVE_HOST = "com.darkskystudios.unshorts";

const sub = String.raw`^https?://([^/?#]+\.)?`;
const end = String.raw`([/?#]|$)`;

export const PLATFORMS = {
  youtube: {
    name: "YouTube",
    home: "https://www.youtube.com/",
    site: [sub + String.raw`(youtube\.com|youtube-nocookie\.com)/`, String.raw`^https?://youtu\.be/`],
    shorts: [sub + String.raw`youtube\.com/(shorts|feed/shorts|(@|channel/|c/|user/)[^/?#]+/shorts)` + end],
  },
  instagram: {
    name: "Instagram",
    home: "https://www.instagram.com/",
    site: [sub + String.raw`instagram\.com/`],
    shorts: [sub + String.raw`instagram\.com/(reels?|explore|[^/?#]+/reels?)` + end],
  },
  facebook: {
    name: "Facebook",
    home: "https://www.facebook.com/",
    site: [sub + String.raw`facebook\.com/`, String.raw`^https?://fb\.watch/`],
    shorts: [
      sub + String.raw`facebook\.com/(reels?|watch|share/r|share/v|[^/?#]+/reels)` + end,
      String.raw`^https?://fb\.watch/`,
    ],
  },
  tiktok: {
    name: "TikTok",
    home: null,
    site: [sub + String.raw`tiktok\.com/`],
    shorts: null, // the whole site is short videos
  },
  snapchat: {
    name: "Snapchat",
    home: "https://www.snapchat.com/",
    site: [sub + String.raw`snapchat\.com/`],
    shorts: [sub + String.raw`snapchat\.com/(spotlight|[^/?#]+/spotlight)` + end],
  },
  x: {
    name: "X / Twitter",
    home: null,
    site: [sub + String.raw`(x\.com|twitter\.com)/`],
    shorts: null,
  },
  linkedin: { name: "LinkedIn", home: null, site: [sub + String.raw`linkedin\.com/`], shorts: null },
  tumblr: { name: "Tumblr", home: null, site: [sub + String.raw`tumblr\.com/`], shorts: null },
  reddit: {
    name: "Reddit",
    home: null,
    site: [sub + String.raw`reddit\.com/`, String.raw`^https?://redd\.it/`],
    shorts: null,
  },
};

// Used when the desktop app can't be reached and nothing is cached.
// Matches a fresh install: nothing is blocked until turned on in the app.
export const DEFAULT_CONFIG = {
  active: false,
  locked: false,
  remainingMs: 0,
  levels: {
    youtube: "shorts",
    instagram: "shorts",
    facebook: "shorts",
    tiktok: "site",
    snapchat: "shorts",
    x: "open",
    linkedin: "open",
    tumblr: "open",
    reddit: "open",
  },
};

export function levelOf(cfg, id) {
  if (!cfg || !cfg.active) return "open";
  return (cfg.levels && cfg.levels[id]) || "open";
}

/** Regexes that are blocked for a platform at its current level. */
export function blockedPatterns(cfg, id) {
  const p = PLATFORMS[id];
  const lvl = levelOf(cfg, id);
  if (lvl === "site") return p.site;
  if (lvl === "shorts") return p.shorts || p.site;
  return [];
}

/** Which platform blocks this URL, or null. */
export function match(cfg, url) {
  for (const id of Object.keys(PLATFORMS)) {
    for (const re of blockedPatterns(cfg, id)) {
      if (new RegExp(re, "i").test(url)) return { id, kind: levelOf(cfg, id) };
    }
  }
  return null;
}

export function blockedPath(id, kind) {
  return `/blocked.html?p=${encodeURIComponent(id)}&k=${encodeURIComponent(kind)}`;
}

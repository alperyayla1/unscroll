//! Every platform Unscroll knows about.
//!
//! `shorts_paths` are path prefixes that only contain short-form video. They are
//! used for the browser URL block-list policy (`domain/path`). The browser
//! extension has its own, richer copy of these rules (regexes + page hiding).
//! Keep both in sync when you add a platform.

use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize)]
pub struct Platform {
    pub id: &'static str,
    pub name: &'static str,
    /// Hostnames written to the hosts file when the whole site is blocked.
    pub hosts: &'static [&'static str],
    /// Base domains for the browser URL block list (subdomains are included).
    pub domains: &'static [&'static str],
    /// Shorts-only path prefixes. Empty = the site has no separable shorts.
    pub shorts_paths: &'static [&'static str],
    /// The whole site is short-form video (TikTok): "Shorts" means "Site".
    pub all_shorts: bool,
}

impl Platform {
    pub fn supports_shorts(&self) -> bool {
        self.all_shorts || !self.shorts_paths.is_empty()
    }
}

pub const PLATFORMS: &[Platform] = &[
    Platform {
        id: "youtube",
        name: "YouTube",
        hosts: &["youtube.com", "www.youtube.com", "m.youtube.com", "youtu.be", "music.youtube.com"],
        domains: &["youtube.com", "youtu.be"],
        shorts_paths: &["youtube.com/shorts", "youtube.com/feed/shorts"],
        all_shorts: false,
    },
    Platform {
        id: "instagram",
        name: "Instagram",
        hosts: &["instagram.com", "www.instagram.com", "i.instagram.com", "graph.instagram.com"],
        domains: &["instagram.com"],
        shorts_paths: &["instagram.com/reels", "instagram.com/reel", "instagram.com/explore"],
        all_shorts: false,
    },
    Platform {
        id: "facebook",
        name: "Facebook",
        hosts: &["facebook.com", "www.facebook.com", "m.facebook.com", "web.facebook.com", "fb.watch"],
        domains: &["facebook.com", "fb.watch"],
        shorts_paths: &["facebook.com/reel", "facebook.com/reels", "facebook.com/watch", "fb.watch"],
        all_shorts: false,
    },
    Platform {
        id: "tiktok",
        name: "TikTok",
        hosts: &["tiktok.com", "www.tiktok.com", "m.tiktok.com", "vm.tiktok.com", "vt.tiktok.com"],
        domains: &["tiktok.com"],
        shorts_paths: &[],
        all_shorts: true,
    },
    Platform {
        id: "snapchat",
        name: "Snapchat",
        hosts: &["snapchat.com", "www.snapchat.com", "web.snapchat.com"],
        domains: &["snapchat.com"],
        shorts_paths: &["snapchat.com/spotlight"],
        all_shorts: false,
    },
    Platform {
        id: "x",
        name: "X / Twitter",
        hosts: &["x.com", "www.x.com", "twitter.com", "www.twitter.com", "mobile.twitter.com", "mobile.x.com"],
        domains: &["x.com", "twitter.com"],
        shorts_paths: &[],
        all_shorts: false,
    },
    Platform {
        id: "linkedin",
        name: "LinkedIn",
        hosts: &["linkedin.com", "www.linkedin.com"],
        domains: &["linkedin.com"],
        shorts_paths: &[],
        all_shorts: false,
    },
    Platform {
        id: "tumblr",
        name: "Tumblr",
        hosts: &["tumblr.com", "www.tumblr.com"],
        domains: &["tumblr.com"],
        shorts_paths: &[],
        all_shorts: false,
    },
    Platform {
        id: "reddit",
        name: "Reddit",
        hosts: &["reddit.com", "www.reddit.com", "old.reddit.com", "new.reddit.com", "redd.it"],
        domains: &["reddit.com", "redd.it"],
        shorts_paths: &[],
        all_shorts: false,
    },
];

pub fn find(id: &str) -> Option<&'static Platform> {
    PLATFORMS.iter().find(|p| p.id == id)
}

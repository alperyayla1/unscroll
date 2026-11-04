const STRINGS = {
  en: {
    shorts_title: "No Shorts today",
    shorts_sub: "Short videos on {p} are blocked by Unshorts.",
    site_title: "{p} is blocked",
    site_sub: "This site is blocked by Unshorts.",
    locked_for: "Locked · {t} left",
    go_home: "Go to {p} home",
    close: "Close tab",
    st_on: "Active",
    st_off: "Off",
    st_locked: "Locked",
    lv_open: "Open",
    lv_shorts: "No Shorts",
    lv_site: "Blocked",
    connected: "Connected to the Unshorts app",
    not_connected: "Unshorts app not found. Using the last known settings.",
    controls_in_app: "Change settings in the Unshorts app.",
  },
  tr: {
    shorts_title: "Bugün shorts yok",
    shorts_sub: "{p} üzerindeki kısa videolar Unshorts tarafından engellendi.",
    site_title: "{p} engelli",
    site_sub: "Bu site Unshorts tarafından engellendi.",
    locked_for: "Kilitli · {t} kaldı",
    go_home: "{p} ana sayfasına git",
    close: "Sekmeyi kapat",
    st_on: "Aktif",
    st_off: "Kapalı",
    st_locked: "Kilitli",
    lv_open: "Açık",
    lv_shorts: "Shorts yok",
    lv_site: "Engelli",
    connected: "Unshorts uygulamasına bağlı",
    not_connected: "Unshorts uygulaması bulunamadı. Son bilinen ayarlar kullanılıyor.",
    controls_in_app: "Ayarları Unshorts uygulamasından değiştir.",
  },
};
export const LANG = (navigator.language || "en").toLowerCase().startsWith("tr") ? "tr" : "en";
export function t(key, vars = {}) {
  let s = STRINGS[LANG][key] || STRINGS.en[key] || key;
  for (const [k, v] of Object.entries(vars)) s = s.replace(`{${k}}`, v);
  return s;
}
export function fmtLeft(ms) {
  const m = Math.max(1, Math.ceil(ms / 60000));
  const d = Math.floor(m / 1440), h = Math.floor((m % 1440) / 60), mm = m % 60;
  const u = LANG === "tr" ? ["g", "sa", "dk"] : ["d", "h", "m"];
  return [d && `${d}${u[0]}`, h && `${h}${u[1]}`, (mm || (!d && !h)) && `${mm}${u[2]}`].filter(Boolean).join(" ");
}

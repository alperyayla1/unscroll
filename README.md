# Unshorts — DarkSky Studios

Windows için hafif bir shorts/doomscrolling engelleyici. Tauri 2 (Rust + HTML), kurulum dosyası ~5 MB.

## Nasıl çalışıyor

| Katman | Ne yapar | Nerede |
|---|---|---|
| **Uygulama** | Ayarlar, süreli kilit, onay ekranı | `src-tauri/`, `ui/` |
| **Arka plan görevi** | Her dakika ve her açılışta SYSTEM olarak `unshorts.exe --enforce` çalışır. Uygulama kapalıyken de kilidi sayar ve engelleri yeniden uygular. | Görev Zamanlayıcı → `DarkSky Studios\Unshorts Enforcer` |
| **Tarayıcı politikaları** | Chrome/Edge/Brave'e `URLBlocklist` yazar (`youtube.com/shorts` gibi). Kilit sırasında gizli pencere, misafir profil ve `chrome://extensions` kapanır, böylece uzantı kapatılamaz. | `HKLM\SOFTWARE\Policies\...` (9001–9999 indeksleri, kullanıcının kendi girdilerine dokunmaz) |
| **hosts dosyası** | "Site" seviyesindeki platformları tamamen kapatır, masaüstü uygulamaları dahil. | `# >>> Unshorts` bloğu |
| **Tarayıcı uzantısı** | Shorts sayfalarını engel sayfasına yönlendirir (sayfa yüklemesi + YouTube/Instagram'ın SPA geçişleri), ana sayfadaki Shorts rafını, sekmeleri ve linkleri gizler. | `extension/` |
| **Native host** | Uzantının uygulamadan ayar okuduğu küçük köprü exe'si (salt okunur, admin istemez). | `host/` |
| **Çekirdek** | Durum modeli, kurallar, kilit saati, imzalama. Platformdan bağımsız, testli. | `core/` |

### Süreli kilit neden atlatılamıyor
- Kilit sırasında ayarlar sadece **sıkılaşabilir**. Ana anahtar kapanmaz, platform seviyesi düşmez.
- Süre, sistem saatiyle değil **Windows açık kalma süresiyle** sayılır. Saati ileri almak işe yaramaz. Yeniden başlatmada internetten saat kontrol edilir.
- `state.json` imzalı ve bir kopyası registry'de. Dosyayı düzenlemek veya silmek kilidi kısaltmaz, uzun olan kopya kazanır.
- Kilit sırasında uninstall reddedilir.
- **Dürüst sınır:** Windows'ta yönetici olan biri registry'yi, görevi ve dosyaları elle silerse her şeyi kaldırabilir. Bu bir irade aracıdır, ebeveyn kontrolü değil. Ama bunu yapmak dakikalar ve bilgi ister, bir tıkla olmaz.

### Platform desteği
| Platform | Shorts | Site |
|---|---|---|
| YouTube | Shorts sayfaları, kanal Shorts sekmesi, ana sayfa/arama/öneri rafları | ✓ |
| Instagram | Reels, Keşfet, akıştaki reels gönderileri | ✓ |
| Facebook | Reels, Watch, fb.watch, akıştaki reels | ✓ |
| TikTok | Tamamı kısa video, site olarak kapanır | ✓ |
| Snapchat | Spotlight | ✓ |
| X, LinkedIn, Tumblr, Reddit | Kısa videolar normal akıştan URL ile ayrılamıyor | ✓ |

Tarayıcılar: Chrome, Edge, Brave, Opera (Opera'da politika katmanı yok, sadece uzantı + hosts).

## Derleme

```powershell
# bir kere
cargo install tauri-cli --version "^2" --locked

# kurulum dosyası → target\release\bundle\nsis\Unshorts_1.0.0_x64-setup.exe
powershell -ExecutionPolicy Bypass -File scripts\build.ps1

# geliştirme (YÖNETİCİ terminalde; uygulama admin ister)
powershell -ExecutionPolicy Bypass -File scripts\build.ps1 -Dev
```

Testler: `cargo test -p unshorts-core`

## Uzantı

Şimdilik "Paketlenmemiş öğe yükle" ile kuruluyor; uygulamadaki **Kur** butonu adımları gösteriyor. ID sabit: `apjejhddoodocpnbmjkamfphefgfmmkc` (manifest'teki `key` sayesinde).

**Mağazaya koyunca:**
1. `extension/` klasörünü zip'leyip Chrome Web Store ve Edge Add-ons'a yükle. Yüklemeden önce `manifest.json`'dan `"key"` satırını sil, mağaza kendi ID'sini verir.
2. Verilen ID'leri `core/src/lib.rs` içindeki `EXTENSION_IDS_CHROME` / `EXTENSION_IDS_EDGE`'e yaz.
3. Yeniden derle. Uygulama uzantıyı politika ile **zorla kurar**: kullanıcı kaldıramaz veya kapatamaz, elle kurulum adımı da kalmaz.

## Değiştirmek istersen
- **İsim:** `tauri.conf.json` (`productName`), `core/src/lib.rs` (`APP_NAME`), `extension/manifest.json`, `ui/index.html`.
- **Yeni platform:** `core/src/platforms.rs` + `extension/rules.js` (+ gerekiyorsa `extension/content.js` CSS'i).
- **Metinler:** `ui/i18n.js`, `extension/text.js` (TR/EN, Windows diline göre otomatik).

# ⚡ SIDM — Smart Internet Download Manager

> **Next-Gen Hybrid High-Speed Download Manager for Windows & Linux**  
> *Engineered in Rust for blazing speed, zero disk thrashing, and universal media grabbing.*

---

## 🌟 Why SIDM is Better Than IDM

| Feature | Legacy IDM | **⚡ SIDM (Next-Gen Hybrid)** |
| :--- | :--- | :--- |
| **End-of-Download Disk Reassembly** | Creates temporary `.tmp` files for every chunk. On 10–50 GB files, it freezes your drive for 5–10 minutes copying chunks together. | **Zero-Copy Sparse Pre-Allocation:** Allocates target file immediately and writes chunks concurrently to disjoint byte offsets. **Instantly ready at 100% (0 ms wait)!** |
| **YouTube & Streaming Stability** | Breaks frequently when YouTube changes cipher/SABR streaming; shows random `videoplayback` parts without audio. | **Hybrid Media Pipeline:** Seamless `yt-dlp` + `ffmpeg -c copy` integration. Detects 8K, 4K, 1080p, 720p, MP3 with embedded metadata & thumbnails. |
| **HLS / DASH Streams** | Downloads segments sequentially at slow speed. | **Parallel Fragment Burst:** Concurrent segment download with up to 16–32 parallel streams. |
| **Private Videos & Cookies** | Fails on private groups, courses, and age-restricted videos. | **Zero-Click Cookie Bridge:** Extension passes session cookies directly to the engine for effortless private media downloads. |
| **Memory Footprint & Speed** | Heavy legacy codebase, high RAM usage over time. | **Rust Engine:** Uses only **15–25 MB RAM**, zero garbage collection, maximum CPU & network utilization. |
| **User Interface** | Outdated Windows 98/XP look. | **2026 Modern Dashboard:** Glassmorphic dark mode, real-time segment progress visualization, live speed meter, and categories. |

---

## 📂 Project Architecture

```text
SIDM/
├── sidm-core/                  # High-performance Rust Core Engine & Local Daemon
│   ├── src/
│   │   ├── main.rs             # CLI and Server Entrypoint
│   │   ├── config.rs           # Core App Configuration
│   │   ├── model.rs            # Data Structures & WebSocket Events
│   │   ├── engine/
│   │   │   ├── classifier.rs   # Smart URL Classifier (Direct vs Media vs HLS)
│   │   │   ├── segmenter.rs    # Dynamic Multi-threaded Range Segmenter (Zero-Copy)
│   │   │   ├── media.rs        # Media Extractor Pipeline (8K/4K/DASH/FFmpeg Muxer)
│   │   │   └── manager.rs      # Central Download Task Coordinator & Event Broadcaster
│   │   └── server/
│   │       ├── mod.rs          # Axum HTTP + WebSocket Server (127.0.0.1:45678)
│   │       ├── routes.rs       # REST API Endpoints
│   │       └── ws.rs           # Real-Time WebSocket Broadcaster
│   └── static/
│       └── index.html          # Embedded Modern Web Dashboard
│
└── sidm-extension/             # Chrome / Edge / Brave Manifest V3 Browser Extension
    ├── manifest.json           # Manifest V3 Specification
    ├── background.js           # Service Worker & Context Menu Handler
    ├── content.js              # Video Sniffer & Floating Download Badge Injector
    ├── content.css             # Glassmorphic In-Page Widget Styling
    ├── popup.html              # Extension Popup Interface
    ├── popup.js                # Popup Controller & Health Poller
    └── icons/                  # 16px, 48px, 128px Icons
```

---

## 🚀 Quick Start Guide

### 🐧 Option A: 1-Click Install on Linux (Zorin OS / Ubuntu / Debian / Mint)

Install the pre-built `.deb` package directly:
```bash
sudo apt install ./dist/sidm_1.1.0_amd64.deb
```
This registers **SIDM** in your system applications launcher. You can launch it from your desktop menu or by typing `sidm` in terminal!

---

### 💻 Option B: Quick Run from Terminal

```bash
# On Linux / macOS:
./start_sidm.sh

# On Windows:
double-click start_sidm.bat
```

The engine starts at:
- **Web Dashboard:** [http://127.0.0.1:45678/](http://127.0.0.1:45678/)
- **Health Endpoint:** [http://127.0.0.1:45678/health](http://127.0.0.1:45678/health)
- **Downloads Directory:** `~/Downloads/SIDM/` (auto-sorted into `Videos/`, `Music/`, `Compressed/`, `Programs/`, `Documents/`)

---

### 🔌 Install Browser Extension (Chrome / Edge / Brave / Opera)

1. Open your browser and navigate to `chrome://extensions/` (or `edge://extensions/`, `brave://extensions/`).
2. Toggle on **"Developer mode"** (top right corner).
3. Click **"Load unpacked"** and select the `sidm-extension` folder inside this repository (or unzip `dist/sidm-extension-v1.1.0.zip`).
4. Done! The **⚡ SIDM Turbo Downloader** floating badge will now automatically appear on Facebook, YouTube, TikTok, and video websites!

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

### 1. Run the SIDM Core Engine & Dashboard

```bash
cd sidm-core
cargo run --release -- server
```
Or use the pre-built binary:
```bash
./sidm-core/target/debug/sidm-core server
```

The server will start at:
- **Web Dashboard:** [http://127.0.0.1:45678/](http://127.0.0.1:45678/)
- **Health Endpoint:** [http://127.0.0.1:45678/health](http://127.0.0.1:45678/health)
- **API Endpoints:** `http://127.0.0.1:45678/api/downloads`

---

### 2. Command-Line Direct Download (Turbo Mode)

You can download any file directly from your terminal using up to 32 parallel threads:

```bash
# Download with 16 connections
./sidm-core/target/debug/sidm-core download "https://example.com/largefile.zip" -c 16

# Inspect available resolutions for a YouTube or Facebook video
./sidm-core/target/debug/sidm-core inspect "https://www.youtube.com/watch?v=dQw4w9WgXcQ"
```

---

### 3. Install the Browser Extension (Chrome / Edge / Brave)

1. Open your browser and go to `chrome://extensions/` (or `edge://extensions/`).
2. Enable **"Developer mode"** (top right corner toggle).
3. Click **"Load unpacked"**.
4. Select the `sidm-extension` folder located inside this repository.
5. Done! The **SIDM Integration Module** icon will appear in your browser toolbar, and a floating **"Download with SIDM"** badge will automatically appear on YouTube and video pages!

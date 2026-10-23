// SIDM Universal Video Sniffer & Floating Button Injector
// Works on YouTube, Facebook, TikTok, Instagram, Twitter/X, and ANY website with HTML5 Video

(function () {
  const PROCESSED_ATTR = "data-sidm-attached";
  let activeDropdown = null;

  function init() {
    scanAndAttachVideos();

    // DOM Observer for SPA navigation (YouTube, Facebook feed scroll, dynamic modals)
    const observer = new MutationObserver(() => {
      scanAndAttachVideos();
    });
    observer.observe(document.body, { childList: true, subtree: true });

    // Periodic safety check for lazy-loaded players
    setInterval(scanAndAttachVideos, 2000);

    // Global click listener to close dropdowns
    document.addEventListener("click", () => {
      if (activeDropdown) {
        activeDropdown.style.display = "none";
        activeDropdown = null;
      }
    });
  }

  function scanAndAttachVideos() {
    const isYouTube = location.hostname.includes("youtube.com");
    const isFacebook = location.hostname.includes("facebook.com");

    if (isYouTube) {
      attachYouTube();
    }

    if (isFacebook) {
      attachFacebook();
    }

    // Generic HTML5 videos across any webpage
    attachGenericVideos();
  }

  // --- 1. YouTube Dedicated Handler ---
  function attachYouTube() {
    // Normal watch player
    const ytPlayer = document.querySelector("#movie_player") || document.querySelector(".html5-video-player");
    if (ytPlayer && !ytPlayer.hasAttribute(PROCESSED_ATTR)) {
      ytPlayer.setAttribute(PROCESSED_ATTR, "true");
      createBadge(ytPlayer, () => window.location.href, "youtube");
    }

    // YouTube Shorts player
    const shortsPlayers = document.querySelectorAll("ytd-shorts-player, #shorts-player");
    shortsPlayers.forEach((sp) => {
      if (!sp.hasAttribute(PROCESSED_ATTR)) {
        sp.setAttribute(PROCESSED_ATTR, "true");
        createBadge(sp, () => window.location.href, "youtube-shorts");
      }
    });
  }

  // --- 2. Facebook Dedicated Handler ---
  function attachFacebook() {
    // Facebook videos in feed, watch, and reels
    const fbVideos = document.querySelectorAll("video");
    fbVideos.forEach((vid) => {
      if (vid.hasAttribute(PROCESSED_ATTR)) return;

      // Find suitable container: closest player wrapper or parent
      const container = vid.closest('[role="dialog"]') ||
                        vid.closest('[data-video-id]') ||
                        vid.parentElement;

      if (container && !container.querySelector(".sidm-floating-badge")) {
        vid.setAttribute(PROCESSED_ATTR, "true");
        createBadge(container, () => {
          // Check if parent has a permalink or use current URL
          const link = container.closest("a[href*='/watch'], a[href*='/reel'], a[href*='/videos/']");
          return link ? link.href : window.location.href;
        }, "facebook");
      }
    });
  }

  // --- 3. Generic HTML5 Video Handler ---
  function attachGenericVideos() {
    const allVideos = document.querySelectorAll("video");
    allVideos.forEach((video) => {
      if (video.hasAttribute(PROCESSED_ATTR)) return;

      // Filter out tiny videos (ads, tracking pixels, avatars < 120px)
      const rect = video.getBoundingClientRect();
      if (rect.width < 140 || rect.height < 100) return;

      const parent = video.parentElement;
      if (!parent || parent.querySelector(".sidm-floating-badge")) return;

      video.setAttribute(PROCESSED_ATTR, "true");

      createBadge(parent, () => {
        // Return video src, source tags, or page URL
        if (video.src && !video.src.startsWith("blob:")) {
          return video.src;
        }
        const source = video.querySelector("source");
        if (source && source.src && !source.src.startsWith("blob:")) {
          return source.src;
        }
        return window.location.href;
      }, "generic");
    });
  }

  // --- Badge Creator ---
  function createBadge(container, getTargetUrl, platform) {
    if (container.querySelector(".sidm-floating-badge")) return;

    // Ensure container has relative/absolute positioning so badge anchors properly
    const computed = window.getComputedStyle(container);
    if (computed.position === "static") {
      container.style.position = "relative";
    }

    const badge = document.createElement("div");
    badge.className = "sidm-floating-badge";
    badge.innerHTML = `
      <div class="sidm-badge-trigger" title="Download video with SIDM">
        <span class="sidm-icon">⚡</span>
        <span class="sidm-label">Download Video</span>
        <span class="sidm-arrow">▼</span>
      </div>
      <div class="sidm-dropdown" style="display: none;">
        <div class="sidm-dropdown-header">
          <span>⚡ SIDM Video Downloader</span>
        </div>
        <div class="sidm-item" data-format="best_8k">
          <span class="sidm-format-icon">🌟</span>
          <div class="sidm-format-info">
            <span class="sidm-format-title">8K / 4K UHD</span>
            <span class="sidm-format-sub">Best Available Stream</span>
          </div>
        </div>
        <div class="sidm-item" data-format="best_1080p">
          <span class="sidm-format-icon">🎬</span>
          <div class="sidm-format-info">
            <span class="sidm-format-title">1080p Full HD</span>
            <span class="sidm-format-sub">60 FPS (Recommended)</span>
          </div>
        </div>
        <div class="sidm-item" data-format="best_720p">
          <span class="sidm-format-icon">⚡</span>
          <div class="sidm-format-info">
            <span class="sidm-format-title">720p HD</span>
            <span class="sidm-format-sub">Fast & Compact</span>
          </div>
        </div>
        <div class="sidm-item" data-format="best_audio">
          <span class="sidm-format-icon">🎵</span>
          <div class="sidm-format-info">
            <span class="sidm-format-title">Audio Only (MP3)</span>
            <span class="sidm-format-sub">320kbps High Quality</span>
          </div>
        </div>
      </div>
    `;

    const trigger = badge.querySelector(".sidm-badge-trigger");
    const dropdown = badge.querySelector(".sidm-dropdown");

    // Toggle dropdown
    trigger.addEventListener("click", (e) => {
      e.stopPropagation();
      e.preventDefault();

      if (activeDropdown && activeDropdown !== dropdown) {
        activeDropdown.style.display = "none";
      }

      const isOpen = dropdown.style.display === "block";
      dropdown.style.display = isOpen ? "none" : "block";
      activeDropdown = isOpen ? null : dropdown;
    });

    // Dropdown items click
    badge.querySelectorAll(".sidm-item").forEach((item) => {
      item.addEventListener("click", (e) => {
        e.stopPropagation();
        e.preventDefault();

        const formatId = item.getAttribute("data-format");
        const targetUrl = getTargetUrl();

        startDownload(targetUrl, formatId);
        dropdown.style.display = "none";
        activeDropdown = null;
      });
    });

    container.appendChild(badge);
  }

  function startDownload(url, formatId) {
    showToast("info", "🚀 Sending to SIDM Engine (32 Turbo Threads)...");

    chrome.runtime.sendMessage(
      {
        action: "SEND_TO_SIDM",
        payload: {
          url: url,
          format_id: formatId,
          connections: 32,
          cookies: document.cookie || ""
        }
      },
      (res) => {
        if (res && res.success && res.data && res.data.success) {
          showToast("success", "✅ Download started in SIDM!");
        } else {
          showToast("error", `❌ SIDM error: ${res ? res.error : "Is SIDM running?"}`);
        }
      }
    );
  }

  function showToast(type, text) {
    let toast = document.getElementById("sidm-toast");
    if (!toast) {
      toast = document.createElement("div");
      toast.id = "sidm-toast";
      document.body.appendChild(toast);
    }
    toast.className = `sidm-toast sidm-toast-${type} sidm-toast-show`;
    toast.textContent = text;

    setTimeout(() => {
      toast.className = "sidm-toast";
    }, 4000);
  }

  // Listen to background toast events
  chrome.runtime.onMessage.addListener((msg) => {
    if (msg.action === "SHOW_TOAST") {
      showToast(msg.type, msg.message);
    }
  });

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", init);
  } else {
    init();
  }
})();

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
    const fbVideos = document.querySelectorAll("video");
    fbVideos.forEach((vid) => {
      if (vid.hasAttribute(PROCESSED_ATTR)) return;

      const container = vid.closest('[role="dialog"]') ||
                        vid.closest('[data-video-id]') ||
                        vid.parentElement;

      if (container && !container.querySelector(".sidm-floating-badge")) {
        vid.setAttribute(PROCESSED_ATTR, "true");
        createBadge(container, () => {
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

      const rect = video.getBoundingClientRect();
      if (rect.width < 140 || rect.height < 100) return;

      const parent = video.parentElement;
      if (!parent || parent.querySelector(".sidm-floating-badge")) return;

      video.setAttribute(PROCESSED_ATTR, "true");

      createBadge(parent, () => {
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

    const computed = window.getComputedStyle(container);
    if (computed.position === "static") {
      container.style.position = "relative";
    }

    const badge = document.createElement("div");
    badge.className = "sidm-floating-badge";
    badge.innerHTML = `
      <div class="sidm-badge-trigger">
        <div class="sidm-btn-main" title="Click to 1-Click Download (1080p Full HD)">
          <span class="sidm-icon">⚡</span>
          <span class="sidm-label">Download Video</span>
        </div>
        <div class="sidm-btn-arrow" title="Choose Resolution (4K, 720p, MP3)">
          <span class="sidm-arrow">▼</span>
        </div>
      </div>
      <div class="sidm-dropdown" style="display: none;">
        <div class="sidm-dropdown-header">
          <span>⚡ Select Video Quality</span>
        </div>
        <div class="sidm-item" data-format="best_8k">
          <span class="sidm-format-icon">🌟</span>
          <div class="sidm-format-info">
            <span class="sidm-format-title">8K / 4K UHD</span>
            <span class="sidm-format-sub">Highest Available Stream</span>
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

    const btnMain = badge.querySelector(".sidm-btn-main");
    const btnArrow = badge.querySelector(".sidm-btn-arrow");
    const dropdown = badge.querySelector(".sidm-dropdown");

    // 1-Click Download when clicking main button
    btnMain.addEventListener("click", (e) => {
      e.stopPropagation();
      e.preventDefault();

      const label = btnMain.querySelector(".sidm-label");
      const origText = label.textContent;
      label.textContent = "⏳ Starting...";

      const targetUrl = getTargetUrl();
      startDownload(targetUrl, "best_1080p", () => {
        label.textContent = "✅ Started!";
        setTimeout(() => { label.textContent = origText; }, 2500);
      }, () => {
        label.textContent = origText;
      });
    });

    // Arrow toggles dropdown
    btnArrow.addEventListener("click", (e) => {
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

  function startDownload(url, formatId, onSuccess, onError) {
    showToast("info", "🚀 Connecting to SIDM Engine (32 Turbo Threads)...");

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
          if (onSuccess) onSuccess();
        } else {
          const errMsg = (res && res.error) ? res.error : "Make sure SIDM Engine is running! Run ./start_sidm.sh";
          showToast("error", `❌ ${errMsg}`);
          if (onError) onError();
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
    }, 4500);
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

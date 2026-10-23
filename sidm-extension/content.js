// SIDM In-Page Video Sniffer & Injector

(function () {
  let activeBadge = null;
  let currentVideoUrl = null;

  function init() {
    checkAndInject();
    // Observe DOM changes for SPA navigation (YouTube, Facebook, etc.)
    const observer = new MutationObserver(() => {
      checkAndInject();
    });
    observer.observe(document.body, { childList: true, subtree: true });
  }

  function checkAndInject() {
    const isYouTube = location.hostname.includes("youtube.com");
    const isFacebook = location.hostname.includes("facebook.com");

    if (isYouTube && location.pathname === "/watch") {
      injectYouTubeWidget();
      return;
    }

    if (isFacebook && (location.pathname.includes("/watch") || location.pathname.includes("/reel"))) {
      injectGenericWidget();
      return;
    }

    // Generic HTML5 video detector
    const video = document.querySelector("video");
    if (video && video.src && !document.getElementById("sidm-floating-badge")) {
      injectGenericWidget();
    }
  }

  function injectYouTubeWidget() {
    const player = document.querySelector("#movie_player") || document.querySelector(".html5-video-player");
    if (!player || document.getElementById("sidm-floating-badge")) return;

    createFloatingBadge(player, window.location.href);
  }

  function injectGenericWidget() {
    const video = document.querySelector("video");
    if (!video || document.getElementById("sidm-floating-badge")) return;

    const parent = video.parentElement || document.body;
    createFloatingBadge(parent, window.location.href);
  }

  function createFloatingBadge(container, mediaUrl) {
    if (document.getElementById("sidm-floating-badge")) return;

    currentVideoUrl = mediaUrl;

    const badge = document.createElement("div");
    badge.id = "sidm-floating-badge";
    badge.innerHTML = `
      <div class="sidm-badge-trigger">
        <span class="sidm-icon">⚡</span>
        <span class="sidm-label">Download with SIDM</span>
        <span class="sidm-arrow">▼</span>
      </div>
      <div class="sidm-dropdown" style="display: none;">
        <div class="sidm-item" data-format="best_8k">
          <span class="sidm-format-icon">🌟</span>
          <div class="sidm-format-info">
            <span class="sidm-format-title">8K / 4K UHD</span>
            <span class="sidm-format-sub">Highest Available Quality</span>
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
            <span class="sidm-format-sub">Fast & Compact Size</span>
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

    // Dropdown toggle
    const trigger = badge.querySelector(".sidm-badge-trigger");
    const dropdown = badge.querySelector(".sidm-dropdown");

    trigger.addEventListener("click", (e) => {
      e.stopPropagation();
      dropdown.style.display = dropdown.style.display === "none" ? "block" : "none";
    });

    document.addEventListener("click", () => {
      dropdown.style.display = "none";
    });

    // Format selection click
    badge.querySelectorAll(".sidm-item").forEach((item) => {
      item.addEventListener("click", (e) => {
        e.stopPropagation();
        const formatId = item.getAttribute("data-format");
        startDownload(mediaUrl, formatId);
        dropdown.style.display = "none";
      });
    });

    container.style.position = container.style.position || "relative";
    container.appendChild(badge);
    activeBadge = badge;
  }

  function startDownload(url, formatId) {
    showToast("info", "Sending download to SIDM Engine...");

    chrome.runtime.sendMessage(
      {
        action: "SEND_TO_SIDM",
        payload: {
          url: url,
          format_id: formatId,
          connections: 16,
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

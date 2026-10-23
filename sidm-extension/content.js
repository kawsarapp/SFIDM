// SIDM Universal Video Sniffer & Floating Button Injector
// Bulletproof support for YouTube, Facebook (Feed, Watch, Reels, Shorts), TikTok, and HTML5 video

(function () {
  let activeDropdown = null;

  function init() {
    scanAndAttach();

    // DOM Observer for SPA navigation & infinite scroll (Facebook Feed, Reels, YouTube Shorts)
    const observer = new MutationObserver(() => {
      scanAndAttach();
    });
    observer.observe(document.body, { childList: true, subtree: true });

    // Periodic safety sweep
    setInterval(scanAndAttach, 1500);

    // Reactive: whenever ANY video starts playing, inject instantly!
    document.addEventListener("play", (e) => {
      if (e.target && e.target.tagName === "VIDEO") {
        handleVideoPlay(e.target);
      }
    }, true);

    document.addEventListener("playing", (e) => {
      if (e.target && e.target.tagName === "VIDEO") {
        handleVideoPlay(e.target);
      }
    }, true);

    // Global click listener to close dropdowns
    document.addEventListener("click", () => {
      if (activeDropdown) {
        activeDropdown.style.display = "none";
        activeDropdown = null;
      }
    });
  }

  function handleVideoPlay(video) {
    const isFacebook = location.hostname.includes("facebook.com");
    const isYouTube = location.hostname.includes("youtube.com");

    if (isYouTube) {
      attachYouTube();
      return;
    }

    if (isFacebook) {
      attachFacebookVideo(video);
      return;
    }

    attachGenericSingleVideo(video);
  }

  function scanAndAttach() {
    const isYouTube = location.hostname.includes("youtube.com");
    const isFacebook = location.hostname.includes("facebook.com");

    if (isYouTube) {
      attachYouTube();
      return;
    }

    if (isFacebook) {
      attachFacebookAll();
      return;
    }

    attachGenericVideos();
  }

  // ==========================================
  // 1. YouTube Handler (Watch & Shorts)
  // ==========================================
  function attachYouTube() {
    const ytPlayer = document.querySelector("#movie_player") || document.querySelector(".html5-video-player");
    if (ytPlayer && !ytPlayer.querySelector(".sidm-floating-badge")) {
      createBadge(ytPlayer, () => window.location.href, "youtube");
    }

    const shortsPlayers = document.querySelectorAll("ytd-shorts-player, #shorts-player, .ytd-reel-video-renderer");
    shortsPlayers.forEach((sp) => {
      if (!sp.querySelector(".sidm-floating-badge")) {
        createBadge(sp, () => window.location.href, "youtube-shorts");
      }
    });
  }

  // ==========================================
  // 2. Facebook Handler (Reels, Feed, Watch)
  // ==========================================
  function attachFacebookAll() {
    const videos = document.querySelectorAll("video");
    videos.forEach((vid) => {
      attachFacebookVideo(vid);
    });
  }

  function attachFacebookVideo(vid) {
    const container = getVisualVideoContainer(vid);
    if (!container || container.querySelector(".sidm-floating-badge")) return;

    createBadge(container, () => getFacebookVideoUrl(vid, container), "facebook");
  }

  function getFacebookVideoUrl(vid, container) {
    // 1. If currently on a Reel page (/reel/123456)
    if (location.pathname.includes("/reel") || location.pathname.includes("/watch")) {
      return window.location.href;
    }

    // 2. Look for post permalink in closest article / container
    const article = vid.closest('[role="article"]') || container.closest('[role="article"]') || container;
    const link = article.querySelector("a[href*='/reel/'], a[href*='/watch'], a[href*='/videos/']");
    if (link && link.href) {
      return link.href;
    }

    // 3. Fallback to direct video src if available
    if (vid.src && !vid.src.startsWith("blob:")) {
      return vid.src;
    }

    return window.location.href;
  }

  // ==========================================
  // 3. Generic HTML5 Videos
  // ==========================================
  function attachGenericVideos() {
    const allVideos = document.querySelectorAll("video");
    allVideos.forEach((vid) => {
      attachGenericSingleVideo(vid);
    });
  }

  function attachGenericSingleVideo(video) {
    const rect = video.getBoundingClientRect();
    if (rect.width < 140 || rect.height < 100) return;

    const container = getVisualVideoContainer(video);
    if (!container || container.querySelector(".sidm-floating-badge")) return;

    createBadge(container, () => {
      if (video.src && !video.src.startsWith("blob:")) {
        return video.src;
      }
      const source = video.querySelector("source");
      if (source && source.src && !source.src.startsWith("blob:")) {
        return source.src;
      }
      return window.location.href;
    }, "generic");
  }

  // ==========================================
  // Container Helper (Finds the visible player box)
  // ==========================================
  function getVisualVideoContainer(vid) {
    let parent = vid.parentElement;
    let best = parent;
    let depth = 0;

    while (parent && depth < 5) {
      const tag = parent.tagName.toUpperCase();
      if (tag === "BODY" || tag === "HTML") break;

      const role = parent.getAttribute("role");
      if (role === "dialog" || role === "main") break;

      const rect = parent.getBoundingClientRect();
      if (rect.width >= 160 && rect.height >= 120) {
        best = parent;
        break;
      }
      parent = parent.parentElement;
      depth++;
    }

    return best || vid.parentElement;
  }

  // ==========================================
  // Badge Component Creation
  // ==========================================
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
        <div class="sidm-btn-arrow" title="Choose Resolution (8K, 4K, 720p, MP3)">
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

    // 1-Click direct download
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

    // Dropdown item click
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

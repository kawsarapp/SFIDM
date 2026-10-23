// SIDM Universal Video Sniffer & Floating Button Injector
// Bulletproof support for YouTube, Facebook (Feed, Watch, Reels, Shorts), TikTok, and HTML5 video
// Includes IDM-Style Live On-Screen Download Progress Card

(function () {
  let activeDropdown = null;
  let activeTaskPoll = null;

  function init() {
    scanAndAttach();

    // DOM Observer for SPA navigation & infinite scroll
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
    // 1. If currently on a Reel page (/reel/123456) or Watch page
    if (location.pathname.includes("/reel/") || location.pathname.includes("/watch")) {
      return window.location.href;
    }

    // 2. Look for post permalink in closest article / container
    const article = vid.closest('[role="article"]') || container.closest('[role="article"]') || container;
    const link = article.querySelector("a[href*='/reel/'], a[href*='/watch'], a[href*='/videos/'], a[href*='fb.watch'], a[href*='/posts/']");
    if (link && link.href) {
      return link.href;
    }

    // 3. Look for timestamp permalink
    const timeLink = article.querySelector("a[href*='/permalink'], a[href*='story_fbid'], a[href*='facebook.com/watch']");
    if (timeLink && timeLink.href) {
      return timeLink.href;
    }

    // 4. Fallback to direct video src if available
    if (vid.src && !vid.src.startsWith("blob:")) {
      return vid.src;
    }
    if (vid.currentSrc && !vid.currentSrc.startsWith("blob:")) {
      return vid.currentSrc;
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
        <div class="sidm-btn-main" title="1-Click Download (1080p Full HD)">
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
        setTimeout(() => { label.textContent = origText; }, 3000);
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

  // ==========================================
  // IDM-Style Live Download Progress Card
  // ==========================================
  let currentActiveTaskId = null;

  function showLiveDownloadCard(taskId, url) {
    currentActiveTaskId = taskId;
    let card = document.getElementById("sidm-download-card");
    const targetParent = document.fullscreenElement || document.body || document.documentElement;

    if (!card) {
      card = document.createElement("div");
      card.id = "sidm-download-card";
      targetParent.appendChild(card);
    } else if (card.parentElement !== targetParent) {
      targetParent.appendChild(card);
    }

    card.classList.add("sidm-card-show");
    card.style.setProperty("display", "block", "important");
    card.style.setProperty("visibility", "visible", "important");

    card.innerHTML = `
      <div class="sidm-card-header">
        <div class="sidm-card-title">
          <span class="sidm-card-pulse"></span>
          <span>⚡ SIDM Turbo Downloader</span>
        </div>
        <div class="sidm-card-actions">
          <a class="sidm-card-link" href="http://127.0.0.1:45678" target="_blank" title="Open SIDM Dashboard">🌐 Dashboard</a>
          <span class="sidm-card-close" title="Dismiss">&times;</span>
        </div>
      </div>
      <div class="sidm-card-body">
        <div class="sidm-card-filename" id="sidmCardFilename">Connecting & analyzing stream...</div>
        <div class="sidm-card-bar-bg">
          <div class="sidm-card-bar-fill" id="sidmCardBar" style="width: 5%"></div>
        </div>
        <div class="sidm-card-stats">
          <span id="sidmCardSpeed">⏳ Initializing 32 Turbo Threads...</span>
          <span id="sidmCardPercent" style="font-weight: 700; color: #38bdf8;">0%</span>
        </div>
      </div>
    `;

    card.querySelector(".sidm-card-close").onclick = () => {
      card.classList.remove("sidm-card-show");
      card.style.setProperty("display", "none", "important");
      if (activeTaskPoll) clearInterval(activeTaskPoll);
      activeTaskPoll = null;
    };

    // Start live poller to update progress bar every 500ms
    if (activeTaskPoll) clearInterval(activeTaskPoll);
    activeTaskPoll = setInterval(() => {
      fetchTaskStatus(currentActiveTaskId);
    }, 500);
  }

  function fetchTaskStatus(taskId) {
    chrome.runtime.sendMessage(
      { action: "GET_TASK_STATUS", taskId: taskId },
      (res) => {
        if (!res || !res.success || !res.task) return;
        updateLiveCard(res.task);
      }
    );
  }

  function updateLiveCard(task) {
    const card = document.getElementById("sidm-download-card");
    if (!card || card.style.display === "none") return;

    const filenameEl = document.getElementById("sidmCardFilename");
    const barEl = document.getElementById("sidmCardBar");
    const speedEl = document.getElementById("sidmCardSpeed");
    const percentEl = document.getElementById("sidmCardPercent");

    if (filenameEl && task.file_name) filenameEl.textContent = task.file_name;

    const percent = task.progress_percent || 0;
    if (barEl) barEl.style.width = `${Math.max(5, percent)}%`;
    if (percentEl) percentEl.textContent = `${percent.toFixed(1)}%`;

    const speedMb = (task.speed_bps / (1024 * 1024)).toFixed(2);
    const currMb = (task.downloaded_bytes / (1024 * 1024)).toFixed(1);
    const totMb = task.total_bytes ? (task.total_bytes / (1024 * 1024)).toFixed(1) : "--";
    const eta = task.eta_seconds ? `${task.eta_seconds}s left` : "";

    if (speedEl) {
      if (task.status === "completed") {
        speedEl.innerHTML = `<span style="color: #34d399; font-weight: 700;">✅ Download Complete! Saved to Downloads/SIDM</span>`;
        if (barEl) {
          barEl.style.width = "100%";
          barEl.style.background = "linear-gradient(90deg, #10b981, #34d399)";
        }
        if (activeTaskPoll) clearInterval(activeTaskPoll);
        activeTaskPoll = null;
      } else if (task.status === "failed") {
        speedEl.innerHTML = `<span style="color: #f87171;">❌ Download error: ${task.error_message || "Failed"}</span>`;
        if (activeTaskPoll) clearInterval(activeTaskPoll);
        activeTaskPoll = null;
      } else {
        speedEl.textContent = `🚀 ${speedMb} MB/s (${currMb} / ${totMb} MB) ${eta} [32 Thds]`;
      }
    }
  }

  function startDownload(url, formatId, onSuccess, onError) {
    // 1. Immediately show live card for instant feedback!
    showLiveDownloadCard(null, url);
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
          const taskId = res.data.task ? res.data.task.id : null;
          if (taskId) {
            currentActiveTaskId = taskId;
          }
          if (onSuccess) onSuccess();
        } else {
          const errMsg = (res && res.error) ? res.error : "Make sure SIDM Engine is running! Run ./start_sidm.sh";
          showToast("error", `❌ ${errMsg}`);
          const speedEl = document.getElementById("sidmCardSpeed");
          if (speedEl) {
            speedEl.innerHTML = `<span style="color: #f87171;">❌ ${errMsg}</span>`;
          }
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

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

    // Smart Clipboard Monitor
    setupClipboardWatcher();
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

    const hasPlaylist = window.location.search.includes("list=") || window.location.pathname.includes("/playlist");
    const playlistBtnHtml = hasPlaylist ? `
        <div class="sidm-btn-playlist" title="Batch Download Entire Playlist with SIDM">
          <span class="sidm-icon">📑</span>
          <span class="sidm-label">Playlist</span>
        </div>
    ` : "";

    const badge = document.createElement("div");
    badge.className = "sidm-floating-badge";
    badge.innerHTML = `
      <div class="sidm-badge-trigger">
        <div class="sidm-btn-main" title="1-Click Download (1080p Full HD)">
          <span class="sidm-icon">⚡</span>
          <span class="sidm-label">Download Video</span>
        </div>
        ${playlistBtnHtml}
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
            <span class="sidm-format-sub">Premiere Pro Ready • MP4</span>
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
    const btnPlaylist = badge.querySelector(".sidm-btn-playlist");
    const btnArrow = badge.querySelector(".sidm-btn-arrow");
    const dropdown = badge.querySelector(".sidm-dropdown");

    if (btnPlaylist) {
      btnPlaylist.addEventListener("click", (e) => {
        e.stopPropagation();
        e.preventDefault();
        openPlaylistModal(window.location.href);
      });
    }

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
  const SIDM_BRIDGE_HTTP = "http://127.0.0.1:45678";
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
    card.classList.remove("sidm-minimized");
    card.style.setProperty("display", "block", "important");
    card.style.setProperty("visibility", "visible", "important");
    card.style.setProperty("height", "auto", "important");

    card.innerHTML = `
      <div class="sidm-card-header" id="sidmCardHeader" title="Drag to move card anywhere">
        <div class="sidm-card-title">
          <span class="sidm-card-pulse"></span>
          <span id="sidmCardHeaderTitle">⚡ SIDM Turbo Downloader</span>
        </div>
        <div class="sidm-card-actions">
          <a class="sidm-card-link" href="${SIDM_BRIDGE_HTTP}" target="_blank" title="Open SIDM Dashboard">🌐 Dashboard</a>
          <span class="sidm-card-btn" id="sidmMinimizeBtn" title="Minimize to compact bar">−</span>
          <span class="sidm-card-btn sidm-card-close" id="sidmCloseBtn" title="Dismiss">&times;</span>
        </div>
      </div>
      <div class="sidm-card-body">
        <div class="sidm-card-filename" id="sidmCardFilename">Connecting & analyzing stream...</div>
        <div class="sidm-card-bar-bg">
          <div class="sidm-card-bar-fill" id="sidmCardBar" style="width: 5%"></div>
        </div>
        <div class="sidm-card-stats" id="sidmCardStats">
          <span id="sidmCardSpeed">⏳ Initializing 32 Turbo Threads...</span>
          <span id="sidmCardPercent" style="font-weight: 700; color: #38bdf8;">0%</span>
        </div>
        <div id="sidmCardCompleteArea" style="display: none;"></div>
      </div>
    `;

    const header = card.querySelector("#sidmCardHeader");
    const minimizeBtn = card.querySelector("#sidmMinimizeBtn");
    const closeBtn = card.querySelector("#sidmCloseBtn");

    // 1. Draggable card
    enableCardDragging(card, header);

    // 2. Minimize / Expand toggle
    minimizeBtn.onclick = (e) => {
      e.stopPropagation();
      toggleCardMinimize(card, minimizeBtn);
    };

    header.onclick = (e) => {
      if (card.classList.contains("sidm-minimized")) {
        toggleCardMinimize(card, minimizeBtn);
      }
    };

    // 3. Close button
    closeBtn.onclick = (e) => {
      e.stopPropagation();
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

  function toggleCardMinimize(card, btn) {
    const isMin = card.classList.toggle("sidm-minimized");
    if (btn) {
      btn.textContent = isMin ? "□" : "−";
      btn.title = isMin ? "Expand card" : "Minimize to compact bar";
    }
  }

  function enableCardDragging(card, header) {
    if (card._dragInit) return;
    card._dragInit = true;

    let isDragging = false;
    let startX = 0, startY = 0, initialLeft = 0, initialTop = 0;

    header.addEventListener("mousedown", (e) => {
      if (e.target.closest(".sidm-card-actions") || e.target.closest("button") || e.target.closest("a")) {
        return;
      }

      isDragging = true;
      startX = e.clientX;
      startY = e.clientY;

      const rect = card.getBoundingClientRect();
      initialLeft = rect.left;
      initialTop = rect.top;

      card.style.setProperty("bottom", "auto", "important");
      card.style.setProperty("right", "auto", "important");
      card.style.setProperty("height", "auto", "important");
      card.style.setProperty("left", `${initialLeft}px`, "important");
      card.style.setProperty("top", `${initialTop}px`, "important");
      header.style.cursor = "grabbing";

      const onMouseMove = (moveEvent) => {
        if (!isDragging) return;
        const dx = moveEvent.clientX - startX;
        const dy = moveEvent.clientY - startY;

        let newLeft = initialLeft + dx;
        let newTop = initialTop + dy;

        const maxLeft = Math.max(10, window.innerWidth - card.offsetWidth - 10);
        const maxTop = Math.max(10, window.innerHeight - card.offsetHeight - 10);

        newLeft = Math.max(10, Math.min(maxLeft, newLeft));
        newTop = Math.max(10, Math.min(maxTop, newTop));

        card.style.setProperty("left", `${newLeft}px`, "important");
        card.style.setProperty("top", `${newTop}px`, "important");
      };

      const onMouseUp = () => {
        isDragging = false;
        header.style.cursor = "grab";
        document.removeEventListener("mousemove", onMouseMove);
        document.removeEventListener("mouseup", onMouseUp);
      };

      document.addEventListener("mousemove", onMouseMove);
      document.addEventListener("mouseup", onMouseUp);
    });
  }

  function fetchTaskStatus(taskId) {
    if (typeof chrome !== "undefined" && chrome.runtime && chrome.runtime.id) {
      try {
        chrome.runtime.sendMessage(
          { action: "GET_TASK_STATUS", taskId: taskId },
          (res) => {
            if (chrome.runtime.lastError || !res || !res.task) {
              return;
            }
            updateLiveCard(res.task);
          }
        );
        return;
      } catch (e) {}
    }
  }

  function openTaskFolder(taskId) {
    if (typeof chrome !== "undefined" && chrome.runtime && chrome.runtime.id) {
      chrome.runtime.sendMessage({ action: "OPEN_TASK_FOLDER", taskId: taskId });
    }
  }

  function openTaskFile(taskId) {
    if (!taskId) return;
    if (typeof chrome !== "undefined" && chrome.runtime && chrome.runtime.id) {
      chrome.runtime.sendMessage({ action: "OPEN_TASK_FILE", taskId: taskId });
    }
  }

  function updateLiveCard(task) {
    const card = document.getElementById("sidm-download-card");
    if (!card || card.style.display === "none") return;

    const titleEl = document.getElementById("sidmCardHeaderTitle");
    const filenameEl = document.getElementById("sidmCardFilename");
    const barEl = document.getElementById("sidmCardBar");
    const speedEl = document.getElementById("sidmCardSpeed");
    const percentEl = document.getElementById("sidmCardPercent");
    const completeArea = document.getElementById("sidmCardCompleteArea");

    if (filenameEl && task.file_name) filenameEl.textContent = task.file_name;

    const percent = task.progress_percent || 0;
    if (barEl) barEl.style.width = `${Math.max(5, percent)}%`;
    if (percentEl) percentEl.textContent = `${percent.toFixed(1)}%`;

    const speedMb = (task.speed_bps / (1024 * 1024)).toFixed(2);
    const currMb = (task.downloaded_bytes / (1024 * 1024)).toFixed(1);
    const totMb = task.total_bytes ? (task.total_bytes / (1024 * 1024)).toFixed(1) : "--";
    const eta = task.eta_seconds ? `${task.eta_seconds}s left` : "";

    // Minimized title status
    if (titleEl) {
      if (task.status === "completed") {
        titleEl.textContent = "✅ Downloaded";
      } else if (card.classList.contains("sidm-minimized")) {
        titleEl.textContent = `⚡ SIDM: ${percent.toFixed(0)}% (${speedMb} MB/s)`;
      } else {
        titleEl.textContent = "⚡ SIDM Turbo Downloader";
      }
    }

    if (task.status === "completed") {
      if (barEl) {
        barEl.style.width = "100%";
        barEl.style.background = "linear-gradient(90deg, #10b981, #34d399)";
      }
      if (speedEl) {
        speedEl.innerHTML = `<span style="color: #34d399; font-weight: 700;">✅ Download Complete! (${currMb} MB)</span>`;
      }
      if (percentEl) {
        percentEl.textContent = "100%";
        percentEl.style.color = "#34d399";
      }

      if (completeArea && completeArea.style.display === "none") {
        playCompletionChime();
        completeArea.style.display = "flex";
        completeArea.style.flexDirection = "column";
        completeArea.style.gap = "8px";

        const savePath = task.save_path || "~/Downloads/SIDM";
        const displayPath = savePath.length > 40 ? "..." + savePath.slice(-37) : savePath;

        completeArea.innerHTML = `
          <div class="sidm-card-location" title="${savePath}">
            <strong>📁 Location:</strong> <span>${displayPath}</span>
          </div>
          <div class="sidm-card-btn-row">
            <button class="sidm-action-btn sidm-btn-folder" id="sidmOpenFolderBtn" title="Open containing folder in file manager">
              <span>📂</span> Open Folder
            </button>
            <button class="sidm-action-btn sidm-btn-play" id="sidmOpenFileBtn" title="Play video file">
              <span>▶️</span> Open Video
            </button>
          </div>
        `;

        const openFolderBtn = document.getElementById("sidmOpenFolderBtn");
        if (openFolderBtn) {
          openFolderBtn.onclick = (e) => {
            e.stopPropagation();
            openTaskFolder(task.id);
            openFolderBtn.innerHTML = "<span>✅</span> Folder Opened!";
            setTimeout(() => { openFolderBtn.innerHTML = "<span>📂</span> Open Folder"; }, 2000);
          };
        }

        const openFileBtn = document.getElementById("sidmOpenFileBtn");
        if (openFileBtn) {
          openFileBtn.onclick = (e) => {
            e.stopPropagation();
            openTaskFile(task.id);
            openFileBtn.innerHTML = "<span>▶️</span> Playing...";
            setTimeout(() => { openFileBtn.innerHTML = "<span>▶️</span> Open Video"; }, 2000);
          };
        }
      }

      if (activeTaskPoll) clearInterval(activeTaskPoll);
      activeTaskPoll = null;
    } else if (task.status === "failed") {
      if (barEl) {
        barEl.style.width = "100%";
        barEl.style.background = "#ef4444";
      }
      if (percentEl) {
        percentEl.textContent = "Error";
        percentEl.style.color = "#ef4444";
      }
      if (speedEl) {
        speedEl.innerHTML = `<span style="color: #f87171; font-size: 11px; word-break: break-word;">❌ ${task.error_message || "Download failed"}</span>`;
      }
      if (activeTaskPoll) clearInterval(activeTaskPoll);
      activeTaskPoll = null;
    } else if (speedEl) {
      speedEl.textContent = `🚀 ${speedMb} MB/s (${currMb} / ${totMb} MB) ${eta} [32 Thds]`;
    }
  }

  function startDownload(url, formatId, onSuccess, onError) {
    // 1. Immediately show live card for instant feedback!
    showLiveDownloadCard(null, url);
    showToast("info", "🚀 Connecting to SIDM Engine (32 Turbo Threads)...");

    const payload = {
      url: url,
      format_id: formatId,
      connections: 32
    };

    sendDownloadRequest(payload, (res) => {
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
    });
  }

  function sendDownloadRequest(payload, callback) {
    if (typeof chrome !== "undefined" && chrome.runtime && chrome.runtime.id) {
      try {
        chrome.runtime.sendMessage(
          { action: "SEND_TO_SIDM", payload: payload },
          (res) => {
            if (chrome.runtime.lastError) {
              const errMsg = chrome.runtime.lastError.message || "";
              if (errMsg.includes("Extension context invalidated")) {
                showToast("warning", "🔄 SIDM extension was reloaded. Please refresh (F5) this tab!");
              } else {
                showToast("error", `⚠️ ${errMsg}`);
              }
              callback({ success: false, error: errMsg });
              return;
            }
            callback(res || { success: false, error: "Empty response from SIDM background" });
          }
        );
        return;
      } catch (err) {
        const errMsg = err.message || "";
        if (errMsg.includes("Extension context invalidated")) {
          showToast("warning", "🔄 SIDM extension was reloaded. Please refresh (F5) this tab!");
        } else {
          showToast("error", `⚠️ ${errMsg}`);
        }
        callback({ success: false, error: errMsg });
        return;
      }
    }

    showToast("error", "⚠️ SIDM Extension is not active. Please refresh (F5) this page.");
    callback({ success: false, error: "Extension not active" });
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

  // ==========================================
  // Audio Completion Chime (IDM-Style Dual Sine Tone)
  // ==========================================
  function playCompletionChime() {
    try {
      const ctx = new (window.AudioContext || window.webkitAudioContext)();
      const now = ctx.currentTime;
      const osc1 = ctx.createOscillator();
      const gain1 = ctx.createGain();
      osc1.type = "sine";
      osc1.frequency.setValueAtTime(587.33, now);
      gain1.gain.setValueAtTime(0.12, now);
      gain1.gain.exponentialRampToValueAtTime(0.01, now + 0.2);
      osc1.connect(gain1);
      gain1.connect(ctx.destination);
      osc1.start(now);
      osc1.stop(now + 0.2);

      const osc2 = ctx.createOscillator();
      const gain2 = ctx.createGain();
      osc2.type = "sine";
      osc2.frequency.setValueAtTime(880, now + 0.12);
      gain2.gain.setValueAtTime(0.16, now + 0.12);
      gain2.gain.exponentialRampToValueAtTime(0.001, now + 0.5);
      osc2.connect(gain2);
      gain2.connect(ctx.destination);
      osc2.start(now + 0.12);
      osc2.stop(now + 0.5);
    } catch (e) {}
  }

  // ==========================================
  // Smart Clipboard Monitor
  // ==========================================
  let activePillTimeout = null;
  function setupClipboardWatcher() {
    document.addEventListener("copy", () => {
      setTimeout(async () => {
        try {
          const text = await navigator.clipboard.readText().catch(() => null);
          if (!text) return;
          const trimmed = text.trim();
          if (!trimmed.startsWith("http://") && !trimmed.startsWith("https://")) return;

          const isMedia = /(youtube\.com|youtu\.be|facebook\.com|fb\.watch|tiktok\.com|instagram\.com|vimeo\.com|twitter\.com|x\.com)/i.test(trimmed);
          const isFile = /\.(mp4|mkv|webm|avi|mov|mp3|m4a|flac|wav|zip|rar|7z|tar|gz|iso|exe|msi|deb|apk|pdf|docx|xlsx)($|\?)/i.test(trimmed);

          if (isMedia || isFile) {
            showClipboardPill(trimmed);
          }
        } catch (e) {}
      }, 150);
    });
  }

  function showClipboardPill(url) {
    const existing = document.getElementById("sidmClipboardPill");
    if (existing) existing.remove();
    if (activePillTimeout) clearTimeout(activePillTimeout);

    let displayUrl = url;
    try {
      const u = new URL(url);
      displayUrl = u.hostname + (u.pathname.length > 25 ? u.pathname.slice(0, 25) + "..." : u.pathname);
    } catch (e) {}

    const pill = document.createElement("div");
    pill.id = "sidmClipboardPill";
    pill.className = "sidm-clipboard-pill";
    pill.innerHTML = `
      <div class="sidm-pill-icon">⚡</div>
      <div class="sidm-pill-body">
        <span class="sidm-pill-title">Download with SIDM?</span>
        <span class="sidm-pill-url" title="${url}">${displayUrl}</span>
      </div>
      <button class="sidm-pill-btn-dl" id="sidmPillDl">⬇️ Download</button>
      <button class="sidm-pill-btn-close" id="sidmPillClose">✕</button>
    `;

    document.body.appendChild(pill);

    pill.querySelector("#sidmPillDl").addEventListener("click", () => {
      startDownload(url, "best_1080p");
      pill.remove();
    });

    pill.querySelector("#sidmPillClose").addEventListener("click", () => {
      pill.remove();
    });

    activePillTimeout = setTimeout(() => {
      if (pill && pill.parentNode) pill.remove();
    }, 7000);
  }

  // ==========================================
  // Playlist Batch Downloader Modal
  // ==========================================
  function openPlaylistModal(url) {
    const existing = document.getElementById("sidmPlaylistModalBackdrop");
    if (existing) existing.remove();

    const backdrop = document.createElement("div");
    backdrop.id = "sidmPlaylistModalBackdrop";
    backdrop.className = "sidm-modal-backdrop";
    backdrop.innerHTML = `
      <div class="sidm-playlist-modal">
        <div class="sidm-modal-header">
          <div class="sidm-modal-title">
            <span>📑</span>
            <span>SIDM Batch Playlist Downloader</span>
          </div>
          <button id="sidmClosePlaylistModal" style="background:none; border:none; color:#94a3b8; font-size:18px; cursor:pointer;">&times;</button>
        </div>
        <div class="sidm-modal-body" id="sidmPlaylistModalBody">
          <div style="text-align: center; padding: 40px 20px; color: #94a3b8;">
            <div style="font-size: 28px; margin-bottom: 8px;">⏳</div>
            <div style="font-weight: 600; color: #f8fafc;">Scanning playlist with SIDM Engine...</div>
            <div style="font-size: 11px; margin-top: 4px;">Fetching video titles, durations, and formats</div>
          </div>
        </div>
        <div class="sidm-modal-footer" id="sidmPlaylistModalFooter" style="display: none;">
          <button id="sidmCancelPlaylistBtn" style="background: rgba(255,255,255,0.08); border:none; color:#f8fafc; padding: 8px 16px; border-radius: 8px; font-size: 12px; cursor:pointer;">Cancel</button>
          <button id="sidmStartBatchBtn" style="background: linear-gradient(135deg, #2563eb, #1d4ed8); border:none; color:#fff; font-weight:700; padding: 8px 20px; border-radius: 8px; font-size: 12px; cursor:pointer; box-shadow: 0 4px 15px rgba(37,99,235,0.4);">
            🚀 Download Selected (<span id="sidmSelectedCount">0</span> Videos)
          </button>
        </div>
      </div>
    `;

    document.body.appendChild(backdrop);

    backdrop.querySelector("#sidmClosePlaylistModal").onclick = () => backdrop.remove();
    backdrop.onclick = (e) => {
      if (e.target === backdrop) backdrop.remove();
    };

    chrome.runtime.sendMessage(
      { action: "INSPECT_PLAYLIST", url: url },
      (resData) => {
        if (!resData || !resData.success || !resData.data) {
          document.getElementById("sidmPlaylistModalBody").innerHTML = `
            <div style="text-align:center; padding: 30px; color: #f87171;">
              <div style="font-size: 24px; margin-bottom: 8px;">❌</div>
              <div style="font-weight:700;">Could not parse playlist</div>
              <div style="font-size:11px; margin-top:4px;">${(resData && resData.error) || "Please check your connection and try again."}</div>
            </div>
          `;
          return;
        }

        const data = resData.data;
        const items = data.items || [];

        let rowsHtml = items.map((it, idx) => `
          <div class="sidm-playlist-row">
            <input type="checkbox" class="sidm-batch-item-cb" data-idx="${idx}" checked style="accent-color: #2563eb; width: 16px; height: 16px; cursor: pointer;">
            <div style="flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-weight: 500;">
              ${idx + 1}. ${it.title}
            </div>
            <div style="font-size: 11px; color: #94a3b8; font-family: monospace;">
              ${it.duration_seconds ? Math.floor(it.duration_seconds / 60) + ":" + String(it.duration_seconds % 60).padStart(2, '0') : ""}
            </div>
          </div>
        `).join("");

        document.getElementById("sidmPlaylistModalBody").innerHTML = `
          <div class="sidm-playlist-toolbar">
            <div>
              <div style="font-weight: 700; color: #38bdf8; font-size: 14px;">${data.title}</div>
              <div style="font-size: 11px; color: #94a3b8;">${items.length} videos detected</div>
            </div>
            <div style="display:flex; align-items:center; gap: 8px;">
              <label style="font-size: 11px; color: #94a3b8;">Format:</label>
              <select id="sidmBatchFormatSelect" style="background:#1e293b; color:#fff; border:1px solid rgba(255,255,255,0.2); border-radius:6px; padding: 4px 8px; font-size: 11px; outline:none;">
                <option value="best_1080p">🎬 1080p Full HD (MP4)</option>
                <option value="best_720p">⚡ 720p HD (MP4)</option>
                <option value="best_8k">🌟 8K / 4K UHD</option>
                <option value="best_audio">🎵 320kbps MP3 (Audio)</option>
              </select>
            </div>
          </div>
          <div style="display: flex; justify-content: space-between; margin-bottom: 8px; font-size: 11px; color: #94a3b8;">
            <label style="cursor: pointer; display: flex; align-items: center; gap: 6px;">
              <input type="checkbox" id="sidmSelectAllCb" checked style="accent-color: #2563eb;"> Select All (${items.length})
            </label>
          </div>
          <div class="sidm-playlist-items" style="max-height: 45vh; overflow-y: auto;">
            ${rowsHtml}
          </div>
        `;

        const footer = document.getElementById("sidmPlaylistModalFooter");
        if (footer) footer.style.display = "flex";

        const updateCount = () => {
          const selected = document.querySelectorAll(".sidm-batch-item-cb:checked").length;
          const countEl = document.getElementById("sidmSelectedCount");
          if (countEl) countEl.textContent = selected;
        };
        updateCount();

        const selectAllCb = document.getElementById("sidmSelectAllCb");
        if (selectAllCb) {
          selectAllCb.onchange = () => {
            document.querySelectorAll(".sidm-batch-item-cb").forEach((cb) => (cb.checked = selectAllCb.checked));
            updateCount();
          };
        }

        document.querySelectorAll(".sidm-batch-item-cb").forEach((cb) => {
          cb.onchange = updateCount;
        });

        document.getElementById("sidmCancelPlaylistBtn").onclick = () => backdrop.remove();

        document.getElementById("sidmStartBatchBtn").onclick = async () => {
          const formatId = document.getElementById("sidmBatchFormatSelect").value;
          const selectedCbs = Array.from(document.querySelectorAll(".sidm-batch-item-cb:checked"));
          if (selectedCbs.length === 0) {
            alert("Please select at least 1 video to download.");
            return;
          }

          const batchPayloads = selectedCbs.map((cb) => {
            const idx = parseInt(cb.getAttribute("data-idx"), 10);
            const it = items[idx];
            return {
              url: it.url,
              format_id: formatId,
              file_name: it.title,
              connections: 16
            };
          });

          const btn = document.getElementById("sidmStartBatchBtn");
          btn.textContent = "⏳ Queueing tasks...";
          btn.disabled = true;

          chrome.runtime.sendMessage(
            { action: "BATCH_DOWNLOAD", payload: { items: batchPayloads } },
            (data) => {
              if (data && data.success) {
                backdrop.remove();
                showToast("success", `⚡ Queued ${data.count} videos in SIDM!`);
                if (data.tasks && data.tasks.length > 0) {
                  currentActiveTaskId = data.tasks[0].id;
                  showLiveDownloadCard(currentActiveTaskId, data.tasks[0].url);
                }
              } else {
                alert("Batch download failed: " + ((data && data.error) || "Unknown error"));
                btn.disabled = false;
              }
            }
          );
        };
      }
    );
  }

  if (typeof chrome !== "undefined" && chrome.runtime && chrome.runtime.onMessage) {
    chrome.runtime.onMessage.addListener((msg) => {
      if (msg.action === "SHOW_TOAST") {
        showToast(msg.type, msg.message);
      }
    });
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", init);
  } else {
    init();
  }
})();

const SIDM_API = "http://127.0.0.1:45678";

const statusBadge = document.getElementById("statusBadge");
const urlInput = document.getElementById("urlInput");
const downloadBtn = document.getElementById("downloadBtn");
const taskList = document.getElementById("taskList");
const saveDirInput = document.getElementById("saveDirInput");
const changeFolderBtn = document.getElementById("changeFolderBtn");
const openFolderBtn = document.getElementById("openFolderBtn");

async function loadConfig() {
  try {
    const res = await fetch(`${SIDM_API}/api/config`);
    const data = await res.json();
    if (data.success && data.default_download_dir) {
      if (saveDirInput) saveDirInput.value = data.default_download_dir;
    }
  } catch (e) {}
}

if (changeFolderBtn) {
  changeFolderBtn.addEventListener("click", async () => {
    changeFolderBtn.textContent = "Picking...";
    try {
      const res = await fetch(`${SIDM_API}/api/config/choose-folder`, { method: "POST" });
      const data = await res.json();
      if (data.success && data.path) {
        if (saveDirInput) saveDirInput.value = data.path;
      }
    } catch (e) {
      alert("Failed to open folder picker: " + e.message);
    } finally {
      changeFolderBtn.textContent = "Change...";
    }
  });
}

if (openFolderBtn) {
  openFolderBtn.addEventListener("click", async () => {
    try {
      await fetch(`${SIDM_API}/api/open-folder`, { method: "POST" });
    } catch (e) {}
  });
}

async function checkHealth() {
  try {
    const res = await fetch(`${SIDM_API}/health`, { signal: AbortSignal.timeout(1500) });
    if (res.ok) {
      statusBadge.textContent = "Engine Online";
      statusBadge.className = "status-badge status-online";
      loadTasks();
      loadConfig();
    } else {
      throw new Error();
    }
  } catch (e) {
    statusBadge.textContent = "Engine Offline";
    statusBadge.className = "status-badge status-offline";
  }
}

async function loadTasks() {
  try {
    const res = await fetch(`${SIDM_API}/api/downloads`);
    const tasks = await res.json();
    renderTasks(tasks);
  } catch (e) {}
}

function renderTasks(tasks) {
  if (!tasks || tasks.length === 0) {
    taskList.innerHTML = `<div style="font-size: 11px; color: #64748b; text-align: center; padding: 12px;">No active downloads.</div>`;
    return;
  }

  taskList.innerHTML = tasks
    .map((t) => {
      const speedMb = (t.speed_bps / (1024 * 1024)).toFixed(2);
      const currMb = (t.downloaded_bytes / (1024 * 1024)).toFixed(1);
      const totMb = t.total_bytes ? (t.total_bytes / (1024 * 1024)).toFixed(1) : "--";
      return `
        <div class="task-item">
          <div class="task-name" title="${t.file_name}">${t.file_name}</div>
          <div class="progress-bar-bg">
            <div class="progress-bar-fill" style="width: ${t.progress_percent || 0}%"></div>
          </div>
          <div class="task-meta">
            <span>${currMb} MB / ${totMb} MB (${t.progress_percent.toFixed(1)}%)</span>
            <span>${speedMb} MB/s | ${t.status}</span>
          </div>
        </div>
      `;
    })
    .join("");
}

downloadBtn.addEventListener("click", async () => {
  const url = urlInput.value.trim();
  if (!url) return;

  downloadBtn.textContent = "Starting...";
  try {
    const res = await fetch(`${SIDM_API}/api/downloads`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        url: url,
        connections: 32
      })
    });
    const data = await res.json();
    if (data.success) {
      urlInput.value = "";
      downloadBtn.textContent = "⚡ Extreme Turbo Download (32 Thds)";
      loadTasks();
    } else {
      alert("Error: " + (data.error || "Failed to start"));
      downloadBtn.textContent = "⚡ Extreme Turbo Download (32 Thds)";
    }
  } catch (e) {
    alert("SIDM Engine is not reachable at " + SIDM_API);
    downloadBtn.textContent = "⚡ Extreme Turbo Download (32 Thds)";
  }
});

function renderUpdateBanner(info) {
  const banner = document.getElementById("updateBanner");
  const verText = document.getElementById("latestVerText");
  const updateBtn = document.getElementById("updateNowBtn");
  if (info && info.updateAvailable) {
    if (verText) verText.textContent = info.tag || `v${info.latestVersion}`;
    if (banner) banner.style.display = "block";
    if (updateBtn) {
      updateBtn.onclick = () => {
        const targetUrl = info.releaseUrl || "https://github.com/kawsarapp/SFIDM/releases/latest";
        if (typeof chrome !== "undefined" && chrome.tabs && chrome.tabs.create) {
          chrome.tabs.create({ url: targetUrl });
        } else {
          window.open(targetUrl, "_blank");
        }
      };
    }
  } else {
    if (banner) banner.style.display = "none";
  }
}

async function checkUpdateBanner() {
  if (typeof chrome !== "undefined" && chrome.storage && chrome.storage.local) {
    chrome.storage.local.get(["sidm_update_info"], (data) => {
      if (data && data.sidm_update_info) {
        renderUpdateBanner(data.sidm_update_info);
      }
    });

    chrome.runtime.sendMessage({ action: "CHECK_FOR_UPDATES" }, (res) => {
      if (res && res.info) {
        renderUpdateBanner(res.info);
      }
    });
  }

  try {
    const res = await fetch(`${SIDM_API}/api/system/update`, { signal: AbortSignal.timeout(1500) });
    if (res.ok) {
      const data = await res.json();
      if (data.update_available) {
        renderUpdateBanner({
          updateAvailable: true,
          tag: data.latest_version,
          releaseUrl: data.download_url || data.release_url || "https://github.com/kawsarapp/SFIDM/releases/latest"
        });
      }
    }
  } catch (e) {}
}

// Auto refresh
checkHealth();
checkUpdateBanner();
setInterval(checkHealth, 2000);

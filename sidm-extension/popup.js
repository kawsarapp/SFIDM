const SIDM_API = "http://127.0.0.1:45678";

const statusBadge = document.getElementById("statusBadge");
const urlInput = document.getElementById("urlInput");
const downloadBtn = document.getElementById("downloadBtn");
const taskList = document.getElementById("taskList");

async function checkHealth() {
  try {
    const res = await fetch(`${SIDM_API}/health`, { signal: AbortSignal.timeout(1500) });
    if (res.ok) {
      statusBadge.textContent = "Engine Online";
      statusBadge.className = "status-badge status-online";
      loadTasks();
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

// Auto refresh
checkHealth();
setInterval(checkHealth, 2000);

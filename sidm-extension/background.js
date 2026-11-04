const SIDM_API = "http://127.0.0.1:45678";
const GITHUB_REPO = "kawsarapp/SFIDM";
const GITHUB_LATEST_API = `https://api.github.com/repos/${GITHUB_REPO}/releases/latest`;

// Semantic version comparator (returns 1 if v1 > v2, -1 if v1 < v2, 0 if equal)
function compareVersions(v1, v2) {
  const p1 = (v1 || "").replace(/^v/i, "").split(".").map(s => parseInt(s, 10) || 0);
  const p2 = (v2 || "").replace(/^v/i, "").split(".").map(s => parseInt(s, 10) || 0);
  const len = Math.max(p1.length, p2.length);
  for (let i = 0; i < len; i++) {
    const num1 = p1[i] || 0;
    const num2 = p2[i] || 0;
    if (num1 > num2) return 1;
    if (num1 < num2) return -1;
  }
  return 0;
}

// Check GitHub for latest releases
async function checkForUpdates(forceNotify = false) {
  try {
    const manifest = chrome.runtime.getManifest();
    const currentVersion = manifest.version;
    const res = await fetch(GITHUB_LATEST_API, {
      headers: { "Accept": "application/vnd.github.v3+json" },
      cache: "no-store"
    });
    if (!res.ok) return null;
    const release = await res.json();
    const latestTag = release.tag_name || "";
    const latestVersion = latestTag.replace(/^v/i, "");

    const updateAvailable = compareVersions(latestVersion, currentVersion) > 0;
    const updateInfo = {
      updateAvailable,
      currentVersion,
      latestVersion,
      tag: latestTag,
      name: release.name || latestTag,
      releaseUrl: release.html_url || `https://github.com/${GITHUB_REPO}/releases/latest`,
      body: release.body || "",
      checkedAt: Date.now()
    };

    await chrome.storage.local.set({ sidm_update_info: updateInfo });

    if (updateAvailable) {
      chrome.action.setBadgeText({ text: "NEW" });
      chrome.action.setBadgeBackgroundColor({ color: "#ef4444" });

      // Notify once per new version
      const stored = await chrome.storage.local.get(["sidm_last_notified_release"]);
      if (stored.sidm_last_notified_release !== latestTag || forceNotify) {
        await chrome.storage.local.set({ sidm_last_notified_release: latestTag });
        chrome.notifications.create("sidm-update-alert", {
          type: "basic",
          iconUrl: "icons/icon128.png",
          title: "🚀 SIDM Update Available!",
          message: `A new version ${latestTag} of SIDM is available! Click here to download.`,
          priority: 2
        });
      }
    } else {
      chrome.action.setBadgeText({ text: "" });
    }
    return updateInfo;
  } catch (err) {
    console.warn("SIDM: Failed to check for updates:", err);
    return null;
  }
}

// Helper: Extract cookies for target URL or domain (including HttpOnly session cookies)
async function getCookiesForUrl(urlStr) {
  if (!urlStr || urlStr.startsWith("blob:") || urlStr.startsWith("data:")) return null;
  // YouTube videos do not need cookies; passing user web cookies triggers YouTube's anti-bot 429 blocks
  if (urlStr.includes("youtube.com") || urlStr.includes("youtu.be")) {
    return null;
  }
  try {
    const urlObj = new URL(urlStr);
    let cookies = await chrome.cookies.getAll({ url: urlStr });
    if (!cookies || cookies.length === 0) {
      cookies = await chrome.cookies.getAll({ domain: urlObj.hostname });
    }
    if (!cookies || cookies.length === 0) return null;
    return cookies.map((c) => `${c.name}=${c.value}`).join("; ");
  } catch (err) {
    console.warn("SIDM: Failed to retrieve cookies for", urlStr, err);
    return null;
  }
}

// Register context menus and update checks on install
chrome.runtime.onInstalled.addListener(() => {
  chrome.contextMenus.create({
    id: "sidm-download-link",
    title: "⚡ Download link with SIDM",
    contexts: ["link", "video", "audio", "image"]
  });

  chrome.contextMenus.create({
    id: "sidm-download-page-video",
    title: "🎬 Download video on page with SIDM",
    contexts: ["page"]
  });

  // Initial update check and alarm setup
  checkForUpdates();
  try {
    chrome.alarms.create("sidm-periodic-update-check", { periodInMinutes: 360 });
  } catch (e) {}
});

// Check for updates on browser startup
chrome.runtime.onStartup.addListener(() => {
  checkForUpdates();
});

// Periodic alarm handler
chrome.alarms.onAlarm.addListener((alarm) => {
  if (alarm.name === "sidm-periodic-update-check") {
    checkForUpdates();
  }
});

// Handle update notification click
chrome.notifications.onClicked.addListener((notificationId) => {
  if (notificationId === "sidm-update-alert") {
    chrome.storage.local.get(["sidm_update_info"], (res) => {
      const url = res.sidm_update_info?.releaseUrl || `https://github.com/${GITHUB_REPO}/releases/latest`;
      chrome.tabs.create({ url });
    });
  }
});

// Handle context menu clicks
chrome.contextMenus.onClicked.addListener(async (info, tab) => {
  const targetUrl = info.linkUrl || info.srcUrl || info.pageUrl;
  if (!targetUrl) return;

  const cookies = await getCookiesForUrl(targetUrl);

  try {
    const res = await fetch(`${SIDM_API}/api/downloads`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        url: targetUrl,
        connections: 16,
        cookies: cookies || undefined
      })
    });
    const data = await res.json();
    if (data.success) {
      notifyTab(tab.id, "success", `SIDM: Download started!`);
    } else {
      notifyTab(tab.id, "error", `SIDM: ${data.error || "Failed"}`);
    }
  } catch (err) {
    notifyTab(tab.id, "error", "SIDM Core is not running. Please start SIDM!");
  }
});

// Listen to messages from content scripts
chrome.runtime.onMessage.addListener((request, sender, sendResponse) => {
  if (request.action === "SEND_TO_SIDM") {
    (async () => {
      let payload = request.payload || {};
      if (!payload.cookies) {
        const targetUrl = payload.url && payload.url.startsWith("http") ? payload.url : sender.tab?.url;
        if (targetUrl) {
          const cookies = await getCookiesForUrl(targetUrl);
          if (cookies) {
            payload.cookies = cookies;
          }
        }
      }

      try {
        const res = await fetch(`${SIDM_API}/api/downloads`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(payload)
        });
        const data = await res.json().catch(() => ({}));
        if (!res.ok) {
          sendResponse({ success: false, error: data.error || `Server HTTP ${res.status}` });
        } else {
          sendResponse({ success: true, data });
        }
      } catch (err) {
        console.warn("SIDM backend unreachable:", err.message);
        sendResponse({
          success: false,
          error: "SIDM Engine is offline! Make sure SIDM is running."
        });
      }
    })();
    return true; // Keep message channel open for async response
  }

  if (request.action === "GET_TASK_STATUS") {
    if (!request.taskId) {
      fetch(`${SIDM_API}/api/downloads`)
        .then((res) => res.json())
        .then((tasks) => {
          const task = tasks && tasks.length > 0 ? tasks[0] : null;
          sendResponse({ success: true, task });
        })
        .catch(() => sendResponse({ success: false }));
      return true;
    }

    fetch(`${SIDM_API}/api/downloads/${request.taskId}`)
      .then((res) => res.json())
      .then((data) => sendResponse({ success: true, task: data.task }))
      .catch((err) => sendResponse({ success: false, error: err.message }));
    return true;
  }

  if (request.action === "OPEN_TASK_FOLDER") {
    const url = request.taskId ? `${SIDM_API}/api/downloads/${request.taskId}/open-folder` : `${SIDM_API}/api/open-folder`;
    fetch(url, { method: "POST" })
      .then((res) => res.json())
      .then((data) => sendResponse(data))
      .catch((err) => sendResponse({ success: false, error: err.message }));
    return true;
  }

  if (request.action === "OPEN_TASK_FILE") {
    if (!request.taskId) return;
    fetch(`${SIDM_API}/api/downloads/${request.taskId}/open-file`, { method: "POST" })
      .then((res) => res.json())
      .then((data) => sendResponse(data))
      .catch((err) => sendResponse({ success: false, error: err.message }));
    return true;
  }

  if (request.action === "INSPECT_MEDIA") {
    (async () => {
      let cookies = request.cookies;
      if (!cookies && request.url) {
        cookies = await getCookiesForUrl(request.url);
      }
      fetch(`${SIDM_API}/api/inspect`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ url: request.url, cookies: cookies || undefined })
      })
        .then((res) => res.json())
        .then((data) => sendResponse({ success: true, data }))
        .catch((err) => sendResponse({ success: false, error: err.message }));
    })();
    return true;
  }

  if (request.action === "INSPECT_PLAYLIST") {
    (async () => {
      let cookies = request.cookies;
      if (!cookies && request.url) {
        cookies = await getCookiesForUrl(request.url);
      }
      fetch(`${SIDM_API}/api/playlist/inspect`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ url: request.url, cookies: cookies || undefined })
      })
        .then((res) => res.json())
        .then((data) => sendResponse(data))
        .catch((err) => sendResponse({ success: false, error: err.message }));
    })();
    return true;
  }

  if (request.action === "GET_CONFIG") {
    fetch(`${SIDM_API}/api/config`)
      .then((res) => res.json())
      .then((data) => sendResponse(data))
      .catch((err) => sendResponse({ success: false, error: err.message }));
    return true;
  }

  if (request.action === "UPDATE_CONFIG") {
    fetch(`${SIDM_API}/api/config`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(request.payload || {})
    })
      .then((res) => res.json())
      .then((data) => sendResponse(data))
      .catch((err) => sendResponse({ success: false, error: err.message }));
    return true;
  }

  if (request.action === "CHOOSE_FOLDER") {
    fetch(`${SIDM_API}/api/config/choose-folder`, { method: "POST" })
      .then((res) => res.json())
      .then((data) => sendResponse(data))
      .catch((err) => sendResponse({ success: false, error: err.message }));
    return true;
  }

  if (request.action === "BATCH_DOWNLOAD") {
    fetch(`${SIDM_API}/api/downloads/batch`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(request.payload)
    })
      .then((res) => res.json())
      .then((data) => sendResponse(data))
      .catch((err) => sendResponse({ success: false, error: err.message }));
    return true;
  }

  if (request.action === "CHECK_FOR_UPDATES") {
    checkForUpdates(request.forceNotify).then((info) => {
      sendResponse({ success: true, info });
    });
    return true;
  }
});

function notifyTab(tabId, type, message) {
  if (!tabId) return;
  chrome.tabs.sendMessage(tabId, {
    action: "SHOW_TOAST",
    type,
    message
  }).catch(() => {});
}

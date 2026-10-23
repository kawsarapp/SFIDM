const SIDM_API = "http://127.0.0.1:45678";

// Register context menus on install
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
});

// Handle context menu clicks
chrome.contextMenus.onClicked.addListener(async (info, tab) => {
  const targetUrl = info.linkUrl || info.srcUrl || info.pageUrl;
  if (!targetUrl) return;

  try {
    const res = await fetch(`${SIDM_API}/api/downloads`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        url: targetUrl,
        connections: 16
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
    fetch(`${SIDM_API}/api/downloads`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(request.payload)
    })
      .then(async (res) => {
        const data = await res.json().catch(() => ({}));
        if (!res.ok) {
          sendResponse({ success: false, error: data.error || `Server HTTP ${res.status}` });
        } else {
          sendResponse({ success: true, data });
        }
      })
      .catch((err) => {
        console.error("SIDM fetch error:", err);
        sendResponse({
          success: false,
          error: "SIDM Engine is offline! Make sure './start_sidm.sh' is running in your terminal."
        });
      });
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

  if (request.action === "INSPECT_MEDIA") {
    fetch(`${SIDM_API}/api/inspect`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ url: request.url })
    })
      .then((res) => res.json())
      .then((data) => sendResponse({ success: true, data }))
      .catch((err) => sendResponse({ success: false, error: err.message }));
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

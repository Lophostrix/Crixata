import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

interface AppStatus {
  status: string;
  version: string;
  model: string;
  model_present: boolean;
  sidecar_present: boolean;
  sidecar_status: string;
  sidecar_ready: boolean;
  local_api_port: number;
  llama_server_port: number;
  cached_policies_count: number;
  zero_telemetry: boolean;
}

interface ProgressPayload {
  percentage: number;
  status: string;
  message: string;
}

async function refreshStatus(): Promise<void> {
  const outputEl = document.getElementById("diagnostic-output");
  const timeEl = document.getElementById("diagnostic-time");
  const apiPortEl = document.getElementById("api-port");
  const apiHealthEl = document.getElementById("api-health");
  const serviceStatusEl = document.getElementById("service-status");
  const statusPillEl = document.getElementById("status-pill");
  const modelStatusEl = document.getElementById("model-status");
  const sidecarStatusEl = document.getElementById("sidecar-status");
  const cachedCountEl = document.getElementById("cached-count");
  const downloadBtn = document.getElementById("download-model-btn") as HTMLButtonElement | null;
  const downloadSummaryEl = document.getElementById("download-summary");

  try {
    const res: AppStatus = await invoke("get_app_status");

    if (outputEl) {
      outputEl.textContent = JSON.stringify(res, null, 2);
    }
    if (timeEl) {
      timeEl.textContent = `Last checked: ${new Date().toLocaleTimeString()}`;
    }
    if (apiPortEl) {
      apiPortEl.textContent = `127.0.0.1:${res.local_api_port}`;
    }
    if (apiHealthEl) {
      apiHealthEl.textContent = "Listening";
      apiHealthEl.className = "metric-value status-good";
    }

    if (modelStatusEl) {
      if (res.model_present) {
        modelStatusEl.textContent = "Downloaded";
        modelStatusEl.className = "metric-value status-good";
      } else {
        modelStatusEl.textContent = "Missing (Download Needed)";
        modelStatusEl.className = "metric-value status-warning";
      }
    }

    if (sidecarStatusEl) {
      if (res.sidecar_ready) {
        sidecarStatusEl.textContent = `Ready (Port ${res.llama_server_port})`;
        sidecarStatusEl.className = "metric-value status-good";
      } else {
        sidecarStatusEl.textContent = res.sidecar_status;
        sidecarStatusEl.className = "metric-value status-warning";
      }
    }

    if (cachedCountEl) {
      cachedCountEl.textContent = `${res.cached_policies_count} policies`;
    }

    if (serviceStatusEl && statusPillEl) {
      if (res.sidecar_ready) {
        serviceStatusEl.textContent = "Inference Ready";
        statusPillEl.className = "status-pill";
      } else if (res.model_present) {
        serviceStatusEl.textContent = "Engine Stopped";
        statusPillEl.className = "status-pill warning";
      } else {
        serviceStatusEl.textContent = "Setup Required";
        statusPillEl.className = "status-pill warning";
      }
    }

    if (downloadBtn && res.model_present && res.sidecar_present) {
      downloadBtn.textContent = "Re-download Model & Engine";
      if (downloadSummaryEl && downloadSummaryEl.textContent === "Ready") {
        downloadSummaryEl.textContent = "Installed";
      }
    }
  } catch (err: any) {
    if (outputEl) {
      outputEl.textContent = `Diagnostics error: ${err?.message || err}`;
    }
    if (apiHealthEl) {
      apiHealthEl.textContent = "Offline";
      apiHealthEl.className = "metric-value status-danger";
    }
  }
}

async function triggerDownload(): Promise<void> {
  const downloadBtn = document.getElementById("download-model-btn") as HTMLButtonElement | null;
  const progressBar = document.getElementById("progress-bar");
  const progressPct = document.getElementById("progress-pct");
  const progressStep = document.getElementById("progress-step");
  const downloadSummaryEl = document.getElementById("download-summary");

  if (downloadBtn) {
    downloadBtn.disabled = true;
    downloadBtn.textContent = "Downloading...";
  }
  if (progressBar) progressBar.style.width = "0%";
  if (progressPct) progressPct.textContent = "0%";
  if (progressStep) progressStep.textContent = "Starting download...";
  if (downloadSummaryEl) downloadSummaryEl.textContent = "In progress...";

  try {
    await invoke("download_model");
  } catch (err: any) {
    if (progressStep) progressStep.textContent = `Error: ${err?.message || err}`;
    if (downloadBtn) {
      downloadBtn.disabled = false;
      downloadBtn.textContent = "Retry Download";
    }
    if (downloadSummaryEl) downloadSummaryEl.textContent = "Error";
  }
}

window.addEventListener("DOMContentLoaded", async () => {
  const checkBtn = document.getElementById("check-status-btn");
  checkBtn?.addEventListener("click", () => {
    refreshStatus();
  });

  const downloadBtn = document.getElementById("download-model-btn");
  downloadBtn?.addEventListener("click", () => {
    triggerDownload();
  });

  // Listen for background download progress events
  try {
    await listen<ProgressPayload>("model-download-progress", (event) => {
      const payload = event.payload;
      const progressBar = document.getElementById("progress-bar");
      const progressPct = document.getElementById("progress-pct");
      const progressStep = document.getElementById("progress-step");
      const downloadSummaryEl = document.getElementById("download-summary");
      const downloadBtn = document.getElementById("download-model-btn") as HTMLButtonElement | null;

      const pct = Math.round(payload.percentage);
      if (progressBar) progressBar.style.width = `${pct}%`;
      if (progressPct) progressPct.textContent = `${pct}%`;
      if (progressStep) progressStep.textContent = payload.message;

      if (payload.status === "completed") {
        if (downloadSummaryEl) downloadSummaryEl.textContent = "Completed";
        if (downloadBtn) {
          downloadBtn.disabled = false;
          downloadBtn.textContent = "Re-download Model & Engine";
        }
        refreshStatus();
      } else if (payload.status === "error") {
        if (downloadSummaryEl) downloadSummaryEl.textContent = "Failed";
        if (downloadBtn) {
          downloadBtn.disabled = false;
          downloadBtn.textContent = "Retry Download";
        }
      }
    });
  } catch (err) {
    console.error("Failed to register progress listener:", err);
  }

  // Initial status check
  refreshStatus();
});

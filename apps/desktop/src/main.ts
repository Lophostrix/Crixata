import { invoke } from "@tauri-apps/api/core";

interface AppStatus {
  status: string;
  version: string;
  model: string;
  zero_telemetry: boolean;
  local_api_port: number;
}

async function refreshStatus(): Promise<void> {
  const outputEl = document.getElementById("diagnostic-output");
  const timeEl = document.getElementById("diagnostic-time");
  const apiHealthEl = document.getElementById("api-health");
  const serviceStatusEl = document.getElementById("service-status");

  try {
    const res: AppStatus = await invoke("get_app_status");
    if (outputEl) {
      outputEl.textContent = JSON.stringify(res, null, 2);
    }
    if (timeEl) {
      timeEl.textContent = `Last verified: ${new Date().toLocaleTimeString()}`;
    }
    if (apiHealthEl) {
      apiHealthEl.textContent = "Listening";
      apiHealthEl.className = "metric-value status-good";
    }
    if (serviceStatusEl) {
      serviceStatusEl.textContent = "Companion Active";
    }
  } catch (err: any) {
    if (outputEl) {
      outputEl.textContent = `Diagnostics error: ${err?.message || err}`;
    }
    if (apiHealthEl) {
      apiHealthEl.textContent = "Error";
      apiHealthEl.className = "metric-value";
    }
  }
}

window.addEventListener("DOMContentLoaded", () => {
  const btn = document.getElementById("check-status-btn");
  btn?.addEventListener("click", () => {
    refreshStatus();
  });

  // Run initial refresh
  refreshStatus();
});

const { invoke } = window.__TAURI__.core;
const { getCurrentWindow } = window.__TAURI__.window;
const { listen } = window.__TAURI__.event;

const appWindow = getCurrentWindow();

const workspaceInput = document.getElementById("workspace");
const workspaceList = document.getElementById("workspace-list");
const providerSelect = document.getElementById("provider");
const modelInput = document.getElementById("model");
const modelList = document.getElementById("model-list");
const form = document.getElementById("launch-form");
const cancelBtn = document.getElementById("cancel");
const errorEl = document.getElementById("error");

let resumeSessionId = null;

function fillDatalist(el, values) {
  el.innerHTML = "";
  for (const v of values) {
    const opt = document.createElement("option");
    opt.value = v;
    el.appendChild(opt);
  }
}

async function refreshModels(provider) {
  const seen = new Set();
  const combined = [];

  const cfg = await invoke("get_defaults");
  for (const m of cfg.model_history) {
    if (!seen.has(m)) {
      seen.add(m);
      combined.push(m);
    }
  }

  const configured = await invoke("configured_models", { provider });
  for (const m of configured) {
    if (!seen.has(m)) {
      seen.add(m);
      combined.push(m);
    }
  }
  fillDatalist(modelList, combined);

  // Remote models arrive asynchronously; merge in once the network call resolves.
  invoke("fetch_remote_models", { provider }).then((remote) => {
    for (const m of remote) {
      if (!seen.has(m)) {
        seen.add(m);
        combined.push(m);
      }
    }
    fillDatalist(modelList, combined);
  });
}

async function resetForm(defaultWorkspace, defaultModel) {
  errorEl.textContent = "";

  const cfg = await invoke("get_defaults");
  const providerNames = await invoke("providers");
  providerSelect.innerHTML = "";
  for (const p of providerNames) {
    const opt = document.createElement("option");
    opt.value = p;
    opt.textContent = p;
    providerSelect.appendChild(opt);
  }
  providerSelect.value = cfg.last_provider;

  const history = await invoke("workspace_history");
  fillDatalist(workspaceList, history);
  workspaceInput.value = defaultWorkspace || history[0] || "";

  await refreshModels(providerSelect.value);
  modelInput.value = defaultModel || cfg.last_model;
}

providerSelect.addEventListener("change", () => refreshModels(providerSelect.value));

listen("picker:reset", () => {
  resumeSessionId = null;
  resetForm();
});

listen("picker:resume", (event) => {
  resumeSessionId = event.payload;
  resetForm();
});

appWindow.onCloseRequested(async (event) => {
  event.preventDefault();
  await appWindow.hide();
});

cancelBtn.addEventListener("click", () => appWindow.hide());

form.addEventListener("submit", async (e) => {
  e.preventDefault();
  const workspace = workspaceInput.value.trim();
  const provider = providerSelect.value;
  const model = modelInput.value.trim();
  if (!workspace) {
    errorEl.textContent = "Workspace path cannot be empty.";
    return;
  }
  try {
    await invoke("launch_workspace", {
      workspace,
      provider,
      model,
      resume: resumeSessionId,
    });
    await appWindow.hide();
  } catch (err) {
    errorEl.textContent = String(err);
  }
});

resetForm();

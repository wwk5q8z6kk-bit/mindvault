const DEFAULT_SETTINGS = {
  apiBaseUrl: 'http://127.0.0.1:9470',
  appBaseUrl: 'http://localhost:5173',
  namespace: 'default',
  authToken: '',
  enrichBeforeSave: true
};

const apiBaseUrlInput = document.querySelector('#apiBaseUrl');
const appBaseUrlInput = document.querySelector('#appBaseUrl');
const namespaceInput = document.querySelector('#namespace');
const authTokenInput = document.querySelector('#authToken');
const enrichBeforeSaveInput = document.querySelector('#enrichBeforeSave');
const saveBtn = document.querySelector('#save');
const statusEl = document.querySelector('#status');

function setStatus(text) {
  statusEl.textContent = text;
}

async function restore() {
  const values = await chrome.storage.sync.get(DEFAULT_SETTINGS);
  apiBaseUrlInput.value = values.apiBaseUrl || DEFAULT_SETTINGS.apiBaseUrl;
  appBaseUrlInput.value = values.appBaseUrl || DEFAULT_SETTINGS.appBaseUrl;
  namespaceInput.value = values.namespace || DEFAULT_SETTINGS.namespace;
  authTokenInput.value = values.authToken || '';
  enrichBeforeSaveInput.checked = values.enrichBeforeSave ?? DEFAULT_SETTINGS.enrichBeforeSave;
}

async function save() {
  const payload = {
    apiBaseUrl: apiBaseUrlInput.value.trim().replace(/\/$/, '') || DEFAULT_SETTINGS.apiBaseUrl,
    appBaseUrl: appBaseUrlInput.value.trim().replace(/\/$/, '') || DEFAULT_SETTINGS.appBaseUrl,
    namespace: namespaceInput.value.trim() || DEFAULT_SETTINGS.namespace,
    authToken: authTokenInput.value.trim(),
    enrichBeforeSave: Boolean(enrichBeforeSaveInput.checked)
  };
  saveBtn.disabled = true;
  await chrome.storage.sync.set(payload);
  saveBtn.disabled = false;
  setStatus('Settings saved.');
}

saveBtn.addEventListener('click', save);
void restore();

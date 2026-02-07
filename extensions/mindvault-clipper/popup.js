async function getActiveTab() {
  const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
  return tab;
}

async function extractSelection(tabId) {
  if (!tabId) return '';
  const [{ result } = {}] = await chrome.scripting.executeScript({
    target: { tabId },
    func: () => {
      const selected = (window.getSelection && window.getSelection().toString()) || '';
      if (selected.trim()) return selected.trim();
      const description = document.querySelector('meta[name="description"]');
      return description?.content?.trim() || '';
    }
  });
  return String(result || '');
}

const urlInput = document.querySelector('#url');
const titleInput = document.querySelector('#title');
const excerptInput = document.querySelector('#excerpt');
const tagsInput = document.querySelector('#tags');
const dedupeInput = document.querySelector('#dedupe');
const createNoteInput = document.querySelector('#createNote');
const saveBtn = document.querySelector('#save');
const selectionBtn = document.querySelector('#selection');
const optionsBtn = document.querySelector('#options');
const statusEl = document.querySelector('#status');

function setStatus(text, tone = '') {
  statusEl.textContent = text;
  statusEl.className = `status ${tone}`.trim();
}

async function prefill() {
  const tab = await getActiveTab();
  if (!tab) return;
  urlInput.value = /^https?:\/\//i.test(tab.url || '') ? tab.url : '';
  titleInput.value = tab.title || '';
  excerptInput.value = await extractSelection(tab.id);
  const defaults = await chrome.storage.sync.get({
    popupDedupeDefault: true,
    popupCreateNoteDefault: false
  });
  dedupeInput.checked = Boolean(defaults.popupDedupeDefault);
  createNoteInput.checked = Boolean(defaults.popupCreateNoteDefault);
}

async function save() {
  const payload = {
    url: urlInput.value.trim(),
    title: titleInput.value.trim() || undefined,
    excerpt: excerptInput.value.trim() || undefined,
    tags: tagsInput.value.trim(),
    clipSource: 'browser-popup',
    dedupe: Boolean(dedupeInput.checked),
    createNote: Boolean(createNoteInput.checked)
  };

  if (!payload.url) {
    setStatus('URL is required', 'error');
    return;
  }

  saveBtn.disabled = true;
  setStatus('Saving...');

  const response = await chrome.runtime.sendMessage({
    type: 'mindvault.import_clip',
    payload
  });

  saveBtn.disabled = false;
  if (response?.ok) {
    const created = response.result?.created ? 'saved' : 'deduped';
    setStatus(`Clip ${created} in MindVault`, 'ok');
    window.setTimeout(() => window.close(), 600);
  } else {
    setStatus(response?.error || 'Failed to save clip', 'error');
  }
}

selectionBtn.addEventListener('click', async () => {
  const tab = await getActiveTab();
  excerptInput.value = await extractSelection(tab?.id);
  setStatus('Selection refreshed');
});

dedupeInput.addEventListener('change', () => {
  void chrome.storage.sync.set({ popupDedupeDefault: Boolean(dedupeInput.checked) });
});

createNoteInput.addEventListener('change', () => {
  void chrome.storage.sync.set({ popupCreateNoteDefault: Boolean(createNoteInput.checked) });
});

saveBtn.addEventListener('click', save);
optionsBtn.addEventListener('click', () => chrome.runtime.openOptionsPage());

void prefill();

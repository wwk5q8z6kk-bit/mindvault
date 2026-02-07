const DEFAULT_SETTINGS = {
  apiBaseUrl: 'http://127.0.0.1:9470',
  appBaseUrl: 'http://localhost:5173',
  namespace: 'default',
  authToken: '',
  enrichBeforeSave: true
};

const CONTEXT_MENU_ID = 'mindvault-save-selection';

function parseTagText(raw) {
  return [...new Set(
    String(raw || '')
      .split(/[\s,]+/)
      .map((item) => item.trim().toLowerCase())
      .filter(Boolean)
  )];
}

async function getSettings() {
  const stored = await chrome.storage.sync.get(DEFAULT_SETTINGS);
  return {
    apiBaseUrl: String(stored.apiBaseUrl || DEFAULT_SETTINGS.apiBaseUrl).replace(/\/$/, ''),
    appBaseUrl: String(stored.appBaseUrl || DEFAULT_SETTINGS.appBaseUrl).replace(/\/$/, ''),
    namespace: String(stored.namespace || DEFAULT_SETTINGS.namespace).trim() || 'default',
    authToken: String(stored.authToken || '').trim(),
    enrichBeforeSave: Boolean(stored.enrichBeforeSave ?? DEFAULT_SETTINGS.enrichBeforeSave)
  };
}

function mergeTagText(base, suggestedTags) {
  const merged = new Set(
    String(base || '')
      .split(/[\s,]+/)
      .map((item) => item.trim().toLowerCase())
      .filter(Boolean)
  );
  for (const tag of suggestedTags || []) {
    const normalized = String(tag || '').trim().toLowerCase();
    if (normalized) merged.add(normalized);
  }
  return [...merged].join(' ');
}

async function enrichClipPayload(payload, settings, headers) {
  if (!settings.enrichBeforeSave || !payload.url) return payload;
  try {
    const response = await fetch(`${settings.apiBaseUrl}/api/v1/clips/enrich`, {
      method: 'POST',
      headers,
      body: JSON.stringify({ url: payload.url })
    });
    if (!response.ok) return payload;
    const enriched = await response.json();
    return {
      ...payload,
      url: enriched.normalized_url || payload.url,
      title: payload.title || enriched.title || undefined,
      excerpt: payload.excerpt || enriched.description || enriched.content_preview || undefined,
      tags: mergeTagText(payload.tags, enriched.suggested_tags)
    };
  } catch {
    return payload;
  }
}

async function importClip(payload) {
  const settings = await getSettings();
  const headers = {
    'Content-Type': 'application/json'
  };
  if (settings.authToken) {
    headers.Authorization = `Bearer ${settings.authToken}`;
  }
  const enrichedPayload = await enrichClipPayload(payload, settings, headers);

  const response = await fetch(`${settings.apiBaseUrl}/api/v1/clips/import`, {
    method: 'POST',
    headers,
    body: JSON.stringify({
      url: enrichedPayload.url,
      title: enrichedPayload.title,
      excerpt: enrichedPayload.excerpt,
      tags: parseTagText(enrichedPayload.tags),
      namespace: enrichedPayload.namespace || settings.namespace,
      clip_source: enrichedPayload.clipSource || 'browser-extension',
      dedupe: enrichedPayload.dedupe !== false,
      create_note: Boolean(enrichedPayload.createNote)
    })
  });

  if (!response.ok) {
    const body = await response.text();
    throw new Error(`MindVault API request failed (${response.status}): ${body}`);
  }

  return await response.json();
}

async function openFallbackHandoff(payload) {
  const settings = await getSettings();
  const params = new URLSearchParams();
  if (payload.url) params.set('url', payload.url);
  if (payload.title) params.set('title', payload.title);
  if (payload.excerpt) params.set('text', payload.excerpt);
  if (payload.tags) params.set('tags', payload.tags);
  await chrome.tabs.create({
    url: `${settings.appBaseUrl}/bookmarks?${params.toString()}`
  });
}

async function captureCurrentSelection(tabId) {
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

async function getActiveTab() {
  const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
  return tab;
}

async function handleSaveFromTab(tab, { forceSelection = false } = {}) {
  if (!tab || !tab.url || !/^https?:\/\//i.test(tab.url)) {
    return { ok: false, error: 'Open an HTTP(S) page before clipping.' };
  }

  const excerpt = forceSelection ? await captureCurrentSelection(tab.id) : '';
  const payload = {
    url: tab.url,
    title: tab.title || undefined,
    excerpt,
    tags: '',
    clipSource: forceSelection ? 'browser-selection' : 'browser-page'
  };

  try {
    const result = await importClip(payload);
    return { ok: true, result };
  } catch (error) {
    await openFallbackHandoff(payload);
    return {
      ok: false,
      error: error instanceof Error ? error.message : 'Unable to connect to MindVault API. Opened handoff page.'
    };
  }
}

chrome.runtime.onInstalled.addListener(async () => {
  chrome.contextMenus.create({
    id: CONTEXT_MENU_ID,
    title: 'Save selection to MindVault',
    contexts: ['selection', 'page']
  });
  const current = await chrome.storage.sync.get(DEFAULT_SETTINGS);
  await chrome.storage.sync.set({ ...DEFAULT_SETTINGS, ...current });
});

chrome.contextMenus.onClicked.addListener(async (info, tab) => {
  if (info.menuItemId !== CONTEXT_MENU_ID || !tab?.id) return;
  const excerpt = String(info.selectionText || '').trim() || (await captureCurrentSelection(tab.id));
  const payload = {
    url: tab.url,
    title: tab.title || undefined,
    excerpt,
    tags: '',
    clipSource: excerpt ? 'context-selection' : 'context-page'
  };

  try {
    await importClip(payload);
  } catch {
    await openFallbackHandoff(payload);
  }
});

chrome.commands.onCommand.addListener(async (command) => {
  if (command !== 'save-selection') return;
  const tab = await getActiveTab();
  const response = await handleSaveFromTab(tab, { forceSelection: true });
  if (!response.ok) {
    console.warn(response.error);
  }
});

chrome.runtime.onMessage.addListener((message, _sender, sendResponse) => {
  if (message?.type !== 'mindvault.import_clip') return undefined;

  importClip(message.payload)
    .then((result) => sendResponse({ ok: true, result }))
    .catch(async (error) => {
      await openFallbackHandoff(message.payload || {});
      sendResponse({
        ok: false,
        error: error instanceof Error ? error.message : 'Failed to import clip'
      });
    });

  return true;
});

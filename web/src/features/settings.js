const SETTINGS_KEYS = {
  apiBase: 'mindvaultApiBaseOverride',
  autoSuggest: 'mindvaultSettingAutoSuggest',
  autoComplete: 'mindvaultSettingAutoComplete',
  autoLinking: 'mindvaultSettingAutoLinking'
};

const DEFAULT_SETTINGS = {
  autoSuggest: true,
  autoComplete: true,
  autoLinking: true
};

function readBool(key, fallback) {
  const raw = localStorage.getItem(key);
  if (raw === null || raw === undefined || raw === '') {
    return fallback;
  }
  return raw === 'true' || raw === '1' || raw === 'yes';
}

function normalizeApiBase(raw) {
  const value = String(raw || '').trim();
  if (!value) {
    return '';
  }
  return value.replace(/\/+$/, '');
}

export const settingsFeature = {
  loadSettings() {
    this.autoSuggestEnabled = readBool(
      SETTINGS_KEYS.autoSuggest,
      DEFAULT_SETTINGS.autoSuggest
    );
    this.autoCompleteEnabled = readBool(
      SETTINGS_KEYS.autoComplete,
      DEFAULT_SETTINGS.autoComplete
    );
    this.autoLinkingEnabled = readBool(
      SETTINGS_KEYS.autoLinking,
      DEFAULT_SETTINGS.autoLinking
    );
    this.apiBaseOverride = normalizeApiBase(localStorage.getItem(SETTINGS_KEYS.apiBase));
  },

  getApiBase() {
    const override = normalizeApiBase(this.apiBaseOverride || localStorage.getItem(SETTINGS_KEYS.apiBase));
    if (override) {
      return override;
    }
    if (window.location.protocol === 'http:' || window.location.protocol === 'https:') {
      return window.location.origin;
    }
    return 'http://127.0.0.1:9470';
  },

  persistSetting(key, value) {
    if (!SETTINGS_KEYS[key]) {
      return;
    }
    localStorage.setItem(SETTINGS_KEYS[key], String(value));
  },

  applySettingsToUi() {
    const autoSuggestToggle = document.getElementById('ai-suggest-auto');
    if (autoSuggestToggle) {
      autoSuggestToggle.checked = Boolean(this.autoSuggestEnabled);
    }

    const settingsAutoSuggest = document.getElementById('settings-auto-suggest');
    if (settingsAutoSuggest) {
      settingsAutoSuggest.checked = Boolean(this.autoSuggestEnabled);
    }

    const settingsAutoComplete = document.getElementById('settings-auto-complete');
    if (settingsAutoComplete) {
      settingsAutoComplete.checked = Boolean(this.autoCompleteEnabled);
    }

    const settingsAutoLink = document.getElementById('settings-auto-link');
    if (settingsAutoLink) {
      settingsAutoLink.checked = Boolean(this.autoLinkingEnabled);
    }

    const apiBaseInput = document.getElementById('settings-api-base');
    if (apiBaseInput) {
      apiBaseInput.value = this.apiBaseOverride || '';
    }
  },

  initSettingsControls() {
    const apiBaseInput = document.getElementById('settings-api-base');
    const applyApiBaseBtn = document.getElementById('settings-apply-api-base');
    const resetApiBaseBtn = document.getElementById('settings-reset-api-base');
    const autoSuggestToggle = document.getElementById('settings-auto-suggest');
    const autoCompleteToggle = document.getElementById('settings-auto-complete');
    const autoLinkToggle = document.getElementById('settings-auto-link');

    this.applySettingsToUi();

    if (applyApiBaseBtn && apiBaseInput) {
      applyApiBaseBtn.addEventListener('click', () => {
        const normalized = normalizeApiBase(apiBaseInput.value);
        this.apiBaseOverride = normalized;
        if (normalized) {
          this.persistSetting('apiBase', normalized);
        } else {
          localStorage.removeItem(SETTINGS_KEYS.apiBase);
        }
        this.apiBase = this.getApiBase();
        this.showNotification('API base updated for this session.', 'success');
      });
    }

    if (resetApiBaseBtn && apiBaseInput) {
      resetApiBaseBtn.addEventListener('click', () => {
        localStorage.removeItem(SETTINGS_KEYS.apiBase);
        this.apiBaseOverride = '';
        this.apiBase = this.getApiBase();
        apiBaseInput.value = '';
        this.showNotification('API base override cleared.', 'info');
      });
    }

    if (autoSuggestToggle) {
      autoSuggestToggle.addEventListener('change', () => {
        this.autoSuggestEnabled = Boolean(autoSuggestToggle.checked);
        this.persistSetting('autoSuggest', this.autoSuggestEnabled);
        if (!this.autoSuggestEnabled) {
          this.renderEditorSuggestions([], null);
        } else {
          this.requestEditorSuggestions(true, { force: true });
        }
        this.applySettingsToUi();
      });
    }

    if (autoCompleteToggle) {
      autoCompleteToggle.addEventListener('change', () => {
        this.autoCompleteEnabled = Boolean(autoCompleteToggle.checked);
        this.persistSetting('autoComplete', this.autoCompleteEnabled);
        if (!this.autoCompleteEnabled) {
          this.clearAutoComplete();
        }
      });
    }

    if (autoLinkToggle) {
      autoLinkToggle.addEventListener('change', () => {
        this.autoLinkingEnabled = Boolean(autoLinkToggle.checked);
        this.persistSetting('autoLinking', this.autoLinkingEnabled);
        if (!this.autoLinkingEnabled) {
          this.clearWikiLinkSuggestions();
          this.hideWysiwygWikiLinkSuggestions();
        }
      });
    }
  }
};

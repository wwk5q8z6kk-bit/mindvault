const SETTINGS_KEYS = {
  apiBase: 'mindvaultApiBaseOverride',
  autoSuggest: 'mindvaultSettingAutoSuggest',
  autoComplete: 'mindvaultSettingAutoComplete',
  autoLinking: 'mindvaultSettingAutoLinking',
  autoSuggestCooldown: 'mindvaultSettingAutoSuggestCooldownMinutes',
  confirmTabReset: 'mindvaultSettingConfirmTabReset',
  activeTab: 'mindvaultSettingActiveTab',
  editorMode: 'mindvaultSettingEditorMode',
  showPowerControls: 'mindvaultSettingShowPowerControls'
};

const DEFAULT_SETTINGS = {
  autoSuggest: true,
  autoComplete: true,
  autoLinking: true,
  autoSuggestCooldownMinutes: 0,
  confirmTabReset: true,
  activeTab: 'nodes-tab',
  editorMode: 'wysiwyg',
  showPowerControls: true
};

function readBool(key, fallback) {
  const raw = localStorage.getItem(key);
  if (raw === null || raw === undefined || raw === '') {
    return fallback;
  }
  return raw === 'true' || raw === '1' || raw === 'yes';
}

function readNumber(key, fallback) {
  const raw = localStorage.getItem(key);
  if (raw === null || raw === undefined || raw === '') {
    return fallback;
  }
  const parsed = Number(raw);
  return Number.isFinite(parsed) ? parsed : fallback;
}

function readString(key, fallback) {
  const raw = localStorage.getItem(key);
  if (raw === null || raw === undefined || raw === '') {
    return fallback;
  }
  return String(raw);
}

function normalizeApiBase(raw) {
  const value = String(raw || '').trim();
  if (!value) {
    return '';
  }
  return value.replace(/\/+$/, '');
}

function normalizeCooldownMinutes(value) {
  const minutes = Math.max(0, Math.min(60, Math.round(Number(value) || 0)));
  return minutes;
}

function formatCooldownLabel(minutes) {
  if (!minutes) {
    return 'Live (no delay)';
  }
  return `Every ${minutes} min`;
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
    this.autoSuggestCooldownMinutes = normalizeCooldownMinutes(
      readNumber(SETTINGS_KEYS.autoSuggestCooldown, DEFAULT_SETTINGS.autoSuggestCooldownMinutes)
    );
    this.confirmResetEnabled = readBool(
      SETTINGS_KEYS.confirmTabReset,
      DEFAULT_SETTINGS.confirmTabReset
    );
    this.activeTabPreference = readString(
      SETTINGS_KEYS.activeTab,
      DEFAULT_SETTINGS.activeTab
    );
    this.editorMode = readString(
      SETTINGS_KEYS.editorMode,
      DEFAULT_SETTINGS.editorMode
    );
    this.showPowerControls = readBool(
      SETTINGS_KEYS.showPowerControls,
      DEFAULT_SETTINGS.showPowerControls
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

    const settingsConfirmReset = document.getElementById('settings-confirm-reset');
    if (settingsConfirmReset) {
      settingsConfirmReset.checked = Boolean(this.confirmResetEnabled);
    }

    const settingsPowerControls = document.getElementById('settings-power-controls');
    if (settingsPowerControls) {
      settingsPowerControls.checked = Boolean(this.showPowerControls);
    }

    const suggestCooldown = document.getElementById('settings-suggest-cooldown');
    const suggestCooldownLabel = document.getElementById('settings-suggest-cooldown-label');
    if (suggestCooldown) {
      suggestCooldown.value = String(this.autoSuggestCooldownMinutes || 0);
    }
    if (suggestCooldownLabel) {
      suggestCooldownLabel.textContent = formatCooldownLabel(this.autoSuggestCooldownMinutes || 0);
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
    const suggestCooldown = document.getElementById('settings-suggest-cooldown');
    const suggestCooldownLabel = document.getElementById('settings-suggest-cooldown-label');

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

    if (settingsConfirmReset) {
      settingsConfirmReset.addEventListener('change', () => {
        this.confirmResetEnabled = Boolean(settingsConfirmReset.checked);
        this.persistSetting('confirmTabReset', this.confirmResetEnabled);
      });
    }

    if (settingsPowerControls) {
      settingsPowerControls.addEventListener('change', () => {
        this.showPowerControls = Boolean(settingsPowerControls.checked);
        this.persistSetting('showPowerControls', this.showPowerControls);
        if (typeof this.applyPowerControlsVisibility === 'function') {
          this.applyPowerControlsVisibility();
        }
      });
    }

    if (suggestCooldown) {
      const updateCooldownLabel = (value) => {
        const normalized = normalizeCooldownMinutes(value);
        if (suggestCooldownLabel) {
          suggestCooldownLabel.textContent = formatCooldownLabel(normalized);
        }
        return normalized;
      };
      updateCooldownLabel(suggestCooldown.value);
      suggestCooldown.addEventListener('input', () => {
        updateCooldownLabel(suggestCooldown.value);
      });
      suggestCooldown.addEventListener('change', () => {
        const normalized = updateCooldownLabel(suggestCooldown.value);
        this.autoSuggestCooldownMinutes = normalized;
        this.persistSetting('autoSuggestCooldown', normalized);
      });
    }
  }
};

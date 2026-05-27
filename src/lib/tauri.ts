// Typed wrappers around the Rust commands exposed by TLiquid's Tauri backend.
// Keeping IPC in one place means the Svelte windows never call `invoke` directly
// and the request/response shapes stay in sync with `src-tauri/src/commands.rs`.
import { invoke } from "@tauri-apps/api/core";

export type ProviderId =
  | "openai"
  | "anthropic"
  | "gemini"
  | "openrouter"
  | "ollama";

export type RoutingMode = "primary" | "secondary" | "explicit";

export interface Language {
  code: string;
  name: string;
}

export interface ProviderMeta {
  id: ProviderId;
  displayName: string;
  /** Whether this provider is selectable in Phase 0. Ollama is Phase 1. */
  available: boolean;
  supportsStreaming: boolean;
}

export interface TranslationRequest {
  sourceText: string;
  routingMode: RoutingMode;
  explicitTargetLanguage: Language | null;
  provider: ProviderId;
  model: string;
  preserveFormatting: boolean;
}

export interface TranslationResponse {
  translatedText: string;
  detectedSourceLanguage: string | null;
  targetLanguage: Language;
  provider: ProviderId;
  model: string;
  latencyMs: number;
}

// Mirrors `config::Settings`. Non-secret settings only — API keys live in the
// macOS Keychain and are referenced by provider id, never returned here.
export interface Settings {
  version: number;
  startup: { enabled: boolean };
  ui: { theme: string; accentColor: string; openResultFrom: string };
  languages: {
    primary: Language;
    secondary: Language | null;
    additional: Array<Language & { enabled: boolean }>;
  };
  shortcuts: {
    translatePrimary: string;
    translateSecondary: string;
    openManualPopup: string;
    enabled: boolean;
  };
  providers: Record<ProviderId, { enabled: boolean; defaultModel: string | null }>;
  defaultProvider: ProviderId;
  defaultModel: string | null;
  output: {
    selectedTextBehavior: string;
    copyOnEnter: boolean;
    replaceSelection: boolean;
  };
  history: { enabled: boolean };
  diagnostics: { enabled: boolean };
}

export const appVersion = (): Promise<string> => invoke("app_version");

export const listProviders = (): Promise<ProviderMeta[]> =>
  invoke("list_providers");

export const getSettings = (): Promise<Settings> => invoke("get_settings");

export const saveSettings = (settings: Settings): Promise<void> =>
  invoke("save_settings", { settings });

/** Absolute path to the non-secret settings file (FR-047, FR-048). */
export const settingsPath = (): Promise<string> => invoke("settings_path");

/** Re-register global shortcuts from settings; returns registration errors (FR-033). */
export const applyShortcuts = (): Promise<string[]> => invoke("apply_shortcuts");

/** Registration errors from the most recent shortcut apply. */
export const shortcutErrors = (): Promise<string[]> => invoke("shortcut_errors");

/** Open macOS System Settings → Privacy & Security → Accessibility (FR-018). */
export const openAccessibilitySettings = (): Promise<void> =>
  invoke("open_accessibility_settings");

/** Local, copy-pasteable diagnostics report (no upload; FR-064/FR-065). */
export const diagnostics = (): Promise<string> => invoke("diagnostics");

export const setProviderKey = (provider: ProviderId, key: string): Promise<void> =>
  invoke("set_provider_key", { provider, key });

export const deleteProviderKey = (provider: ProviderId): Promise<void> =>
  invoke("delete_provider_key", { provider });

export const hasProviderKey = (provider: ProviderId): Promise<boolean> =>
  invoke("has_provider_key", { provider });

export const testProviderKey = (
  provider: ProviderId,
  key: string,
): Promise<boolean> => invoke("test_provider_key", { provider, key });

/** Validate the provider's already-saved Keychain key (FR-040). */
export const testProviderConnection = (
  provider: ProviderId,
): Promise<boolean> => invoke("test_provider_connection", { provider });

export const listProviderModels = (provider: ProviderId): Promise<string[]> =>
  invoke("list_provider_models", { provider });

export const translate = (
  request: TranslationRequest,
): Promise<TranslationResponse> => invoke("translate", { request });

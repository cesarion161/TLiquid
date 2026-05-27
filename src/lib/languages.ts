// Static list of selectable languages for the settings UI (P0-006).
//
// Phase 0 uses a fixed, provider-neutral list rather than a per-provider one:
// the actual translation target is just a name passed into the prompt, so any
// language the model knows works. `code` is a BCP-47-ish short code stored in
// settings; `name` is the English display/prompt name (PRD §13.7 — native
// names can be added later).
import type { Language } from "./tauri";

export const COMMON_LANGUAGES: readonly Language[] = [
  { code: "en", name: "English" },
  { code: "es", name: "Spanish" },
  { code: "fr", name: "French" },
  { code: "de", name: "German" },
  { code: "it", name: "Italian" },
  { code: "pt", name: "Portuguese" },
  { code: "nl", name: "Dutch" },
  { code: "ru", name: "Russian" },
  { code: "uk", name: "Ukrainian" },
  { code: "pl", name: "Polish" },
  { code: "cs", name: "Czech" },
  { code: "sv", name: "Swedish" },
  { code: "da", name: "Danish" },
  { code: "no", name: "Norwegian" },
  { code: "fi", name: "Finnish" },
  { code: "tr", name: "Turkish" },
  { code: "el", name: "Greek" },
  { code: "he", name: "Hebrew" },
  { code: "ar", name: "Arabic" },
  { code: "fa", name: "Persian" },
  { code: "hi", name: "Hindi" },
  { code: "bn", name: "Bengali" },
  { code: "ur", name: "Urdu" },
  { code: "th", name: "Thai" },
  { code: "vi", name: "Vietnamese" },
  { code: "id", name: "Indonesian" },
  { code: "ms", name: "Malay" },
  { code: "ja", name: "Japanese" },
  { code: "ko", name: "Korean" },
  { code: "zh", name: "Chinese (Simplified)" },
  { code: "zh-Hant", name: "Chinese (Traditional)" },
  { code: "ro", name: "Romanian" },
  { code: "hu", name: "Hungarian" },
  { code: "bg", name: "Bulgarian" },
  { code: "sr", name: "Serbian" },
  { code: "hr", name: "Croatian" },
  { code: "sk", name: "Slovak" },
  { code: "sl", name: "Slovenian" },
  { code: "lt", name: "Lithuanian" },
  { code: "lv", name: "Latvian" },
  { code: "et", name: "Estonian" },
  { code: "ca", name: "Catalan" },
  { code: "tl", name: "Filipino" },
  { code: "sw", name: "Swahili" },
];

/** Look up a language by code, falling back to a `{code, name: code}` shape. */
export function languageByCode(code: string): Language {
  return COMMON_LANGUAGES.find((l) => l.code === code) ?? { code, name: code };
}

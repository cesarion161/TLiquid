# PRD: TLiquid — macOS-First BYOK LLM Translator

**Product name:** TLiquid  
**Document type:** Product Requirements Document  
**Version:** 0.3  
**Date:** 2026-05-26  
**Status:** Draft  
**Phase 0 platform scope:** macOS only  
**Long-term platform direction:** macOS first, then Windows, then Linux  
**Primary technical direction:** Rust + Tauri + Svelte  
**Distribution model:** Direct distribution, open source

---

## 1. Product summary

TLiquid is a lightweight, open-source, macOS-first desktop translator that runs silently in the background, exposes a persistent menu-bar icon, and lets users translate selected text from any macOS application with minimal friction.

The core Phase 0 workflow is:

```text
Select text → press global hotkey → translation appears → press Enter to copy
```

TLiquid is not intended to be a full chat application. It is a small system utility for fast LLM-powered translation.

Phase 0 is strictly scoped to macOS because that is the only platform currently available for direct testing and validation. The app will still be built with Tauri and Rust so the architecture remains portable, but Windows and Linux functionality is explicitly outside Phase 0 acceptance criteria.

Phase 0 focuses on a local-first, bring-your-own-key model. Users provide their own OpenAI, Anthropic, Gemini, or OpenRouter API key. TLiquid provides the desktop shell, menu-bar utility, selected-text workflow, model routing, primary/secondary language logic, and translation UI.

The app is fully open source. Local BYOK usage is not artificially limited by number of languages. Future monetization should come from hosted cloud convenience, managed LLM usage, account sync, cloud profiles, and advanced services rather than restricting local open-source functionality.

---

## 2. Problem statement

Current LLM translation workflows are too slow for frequent desktop use.

Typical user flow today:

```text
copy text → open browser/chat app → paste text → write prompt → wait → copy result → return to original app
```

This is excessive for small translation tasks.

Existing tools are often one or more of the following:

1. Too heavyweight.
2. Subscription-only.
3. Not transparent about provider usage.
4. Not open source.
5. Browser/chat-first rather than desktop-first.
6. Poorly integrated into system-wide text selection.
7. Too resource-heavy for an always-running utility.

TLiquid solves this by behaving like a native macOS utility: always available, tiny, private-friendly, keyboard-driven, and provider-neutral.

---

## 3. Product goals

### 3.1 Phase 0 goals: macOS MVP

1. Ship an open-source macOS desktop app.
2. Use Rust + Tauri + Svelte as the default stack.
3. Keep the codebase portable where reasonable, but verify and accept only macOS behavior.
4. Minimize RAM, CPU, and disk footprint.
5. Run in the background.
6. Avoid occupying normal Dock space while idle where macOS allows this.
7. Show a persistent macOS menu-bar icon.
8. Provide a simple menu-bar panel for manual translation.
9. Support selected-text translation from macOS apps using a global hotkey.
10. Provide primary/secondary language behavior so users rarely need to manually choose a target language.
11. Support unlimited local BYOK target languages.
12. Require one primary language; English is the default.
13. Allow optional secondary language.
14. Support provider API keys for OpenAI, Anthropic, Gemini, and OpenRouter.
15. Let users select a default provider/model from configured providers.
16. Store secrets securely using macOS Keychain where possible.
17. Provide settings through UI and editable local non-secret configuration file.
18. Do not send any user data to TLiquid servers in Phase 0.
19. Do not include telemetry network calls in Phase 0.
20. Skip right-click integration in Phase 0.
21. Produce an installable macOS build that can be manually installed and used by the project owner.

### 3.2 Phase 1 goals: macOS polish and local models

1. Add startup-on-login behavior, configurable and ON by default after user consent/onboarding.
2. Add configurable global shortcuts.
3. Add configurable output behavior:
   - show popup
   - copy to clipboard
   - optionally replace selected text later if technically safe
4. Add Ollama/local model support.
5. Add macOS right-click integration through macOS Services or equivalent native mechanism.
6. Improve macOS selected-text capture reliability.
7. Add shortcut conflict detection and remediation.
8. Improve macOS permissions onboarding.
9. Improve install/notarization/signing flow where feasible.
10. Add streaming translation output — provider deltas (SSE for cloud providers, NDJSON for Ollama) surfaced incrementally in the panel. Deferred from Phase 0 (§15.5, §24).

### 3.3 Phase 2 goals: Windows and hosted cloud

1. Add Windows support as a verified product target.
2. Add hosted LLM proxy option.
3. Add paid cloud tier for users who do not want to configure provider keys.
4. Add account/licensing infrastructure for hosted cloud features.
5. Add auto-update checks once per day and on startup.
6. Add user-triggered update installation from settings.
7. Add background update download and relaunch flow.
8. Add Windows right-click integration if technically practical.
9. Add optional anonymous diagnostics if and only if backend infrastructure exists and the user explicitly opts in.
10. Add translation history as an optional local-first feature or cloud feature, depending on privacy/product decision.

### 3.4 Phase 3+ goals: Linux and advanced product surface

1. Add Linux support as a verified product target.
2. Add Linux right-click integration where practical, likely desktop-environment-specific.
3. Add optional translation history.
4. Add translation memory.
5. Add cloud profile sync.
6. Add organization/team settings.
7. Add advanced translation modes.
8. Add richer platform-specific native polish if strategically useful.

---

## 4. Non-goals

The product will not initially:

1. Support Windows as a verified Phase 0 target.
2. Support Linux as a verified Phase 0 target.
3. Replace full document translation tools.
4. Translate images or screenshots with OCR.
5. Provide human translation review.
6. Store user translation history by default.
7. Send user translations to TLiquid servers in Phase 0.
8. Provide grammar correction, summarization, rewriting, or explanation features in the first MVP.
9. Provide right-click integration in Phase 0.
10. Bundle our own LLM usage in Phase 0.
11. Implement heavy visual customization or platform-specific native UI polish before the core workflow is reliable.

---

## 5. Target users

### 5.1 Primary Phase 0 user: macOS power desktop user

A developer, researcher, student, founder, analyst, or knowledge worker using macOS who frequently reads and writes in multiple languages and already has, or is comfortable creating, API keys for LLM providers.

Needs:

- Fast translation from any macOS app.
- Minimal resource usage.
- BYOK provider configuration.
- Keyboard-first workflow.
- No browser context switch.
- Privacy-friendly local-first behavior.
- Open-source trust.

### 5.2 Secondary future user: cross-platform consumer desktop user

A Windows or Linux user, or a consumer macOS user, who needs frequent translation but does not want to manage provider keys.

Needs:

- Simple install.
- Hosted translation.
- Clear pricing.
- Minimal settings.
- Native-feeling UI.

This user is mainly targeted from Phase 2 onward.

---

## 6. Core positioning

TLiquid should feel like a system utility, not a chat app.

The user should think:

> “Select text, press shortcut, get translation.”

Not:

> “Open an AI app, paste text, write a prompt, copy the answer.”

Positioning statement:

> TLiquid is a tiny open-source BYOK LLM translator that lives in your macOS menu bar and translates selected text instantly.

---

## 7. Product principles

1. **macOS-first execution:** Phase 0 must produce a working, installable, testable macOS app.
2. **Portable architecture, verified later:** Tauri keeps future Windows/Linux support possible, but unverified platforms are not Phase 0 commitments.
3. **Zero-friction invocation:** selected text should be translatable without manual copy/paste into the app.
4. **Background-first:** the app lives in the menu bar and stays out of the Dock when idle where possible.
5. **Open-source first:** local BYOK functionality should be fully inspectable and not artificially crippled.
6. **Local-first:** BYOK calls go directly from the user’s machine to the configured provider.
7. **Small footprint:** optimize for low memory, low CPU, and small binary size.
8. **Keyboard-first:** every critical action should have a shortcut path.
9. **Primary/secondary language simplicity:** most users translate back and forth between one main pair of languages; the product should optimize for this.
10. **Native-feeling UI:** use standard macOS affordances where possible without delaying MVP.
11. **Provider-neutral:** OpenAI, Anthropic, Gemini, OpenRouter, and later Ollama/local models should be treated as interchangeable translation backends.
12. **Transparent privacy:** no hidden analytics, no hidden server calls, no translation storage by default.
13. **Cloud monetization, not local restriction:** paid value should come from hosted convenience and cloud services, not from blocking local open-source functionality.

---

## 8. Phase scope

| Feature | Phase 0: macOS MVP | Phase 1: macOS polish | Phase 2: Windows + cloud | Phase 3+: Linux + advanced |
|---|---:|---:|---:|---:|
| macOS desktop app | Yes | Improve | Improve | Improve |
| Windows verified app | No | No | Yes | Improve |
| Linux verified app | No | No | No | Yes |
| Open-source app | Yes | Yes | Yes | Yes |
| Rust + Tauri implementation | Yes | Yes | Yes | Yes |
| Svelte UI | Yes | Yes | Yes | Yes |
| macOS menu-bar background mode | Yes | Improve | Improve | Improve |
| No Dock presence while idle | Yes, where possible | Improve | Improve | Improve |
| Manual translation panel | Yes | Improve | Improve | Improve |
| Global hotkey for the panel | Yes | Configurable | Configurable | Configurable |
| Global hotkey for selected text translation | macOS only | Improve | Windows added | Linux added |
| Primary language | Yes | Improve | Improve | Improve |
| Secondary language | Yes | Improve | Improve | Improve |
| Unlimited local BYOK languages | Yes | Yes | Yes | Yes |
| OpenAI provider | Yes | Improve | Improve | Improve |
| Anthropic provider | Yes | Improve | Improve | Improve |
| Gemini provider | Yes | Improve | Improve | Improve |
| OpenRouter provider | Yes | Improve | Improve | Improve |
| Ollama/local models | No | Yes | Improve | Improve |
| Hosted LLM proxy | No | No | Yes | Improve |
| Default model dropdown | Yes | Improve | Improve | Improve |
| Startup on login | No or manual | Yes | Improve | Improve |
| Auto-update check | No | No | Yes | Improve |
| Update now button | No | No | Yes | Improve |
| Anonymous diagnostics | No network telemetry | No network telemetry unless backend exists | Optional opt-in | Optional opt-in |
| Right-click integration | No | macOS | Windows | Linux best effort |
| Translation history | No | No | Optional | Improve |
| Installable build | macOS | macOS improved | macOS + Windows | macOS + Windows + Linux |

---

## 9. Language model and translation behavior

### 9.1 Language configuration

Every user has:

1. One required primary language.
2. One optional secondary language.
3. Any number of additional target languages.

Default configuration:

```text
Primary language: English
Secondary language: unset
```

If user adds a secondary language, TLiquid can support fast bidirectional translation without requiring the user to choose a target language for most interactions.

Example configuration:

```text
Primary language: English
Secondary language: Spanish
Additional languages: Russian, German, French
```

### 9.2 Primary translation hotkey

Default hotkey:

```text
Cmd + Shift + T
```

Behavior:

```text
If source language is not primary:
    translate to primary language

If source language is primary and secondary language exists:
    translate to secondary language

If source language is primary and secondary language does not exist:
    open compact target-language chooser or show setup prompt
```

Examples:

Primary = English  
Secondary = Spanish

| Source text language | Result |
|---|---|
| Spanish | English |
| Russian | English |
| German | English |
| English | Spanish |

### 9.3 Secondary translation hotkey

Default hotkey:

```text
Cmd + Shift + Option + T
```

Behavior:

```text
Always translate to secondary language
```

If secondary language is not configured, TLiquid opens settings or a compact setup prompt.

### 9.4 Additional languages

Additional languages are available from settings and optionally from the translate view in the panel.

Phase 0:

- User can configure unlimited languages.
- Primary and secondary are the main fast paths.
- Additional languages can be chosen manually in the panel.

Phase 1:

- User can assign custom shortcuts to additional languages.

### 9.5 Source language auto-detection

The model should auto-detect the source language.

The user should not have to choose source language manually.

Translation prompt should specify:

```text
Detect the source language automatically.
Translate into the target language.
If source language is already the target language, apply the fallback rule provided by the app.
Return only the translation.
```

### 9.6 Ambiguity handling

Short text can be ambiguous.

Examples:

```text
Hola
OK
No
Si
May
```

Phase 0 behavior:

- Make best effort.
- Do not block translation.
- Do not ask user to clarify unless translation fails.

Future behavior:

- Optional compact language override in the panel.

---

## 10. Core UX flows

### 10.1 First launch

On first launch:

1. App opens minimal setup window.
2. User chooses primary language.
3. Default primary language is English.
4. User optionally chooses secondary language.
5. User selects provider: OpenAI, Anthropic, Gemini, or OpenRouter.
6. User pastes API key.
7. App validates key if feasible.
8. User selects default model.
9. App shows default shortcuts.
10. App requests necessary macOS permissions only when needed.
11. App minimizes to menu bar.

If user skips provider setup:

- App remains installed but translation is disabled.
- The panel shows `Configure provider`.

### 10.2 Translate selected text by primary hotkey

Trigger:

```text
User selects text in any macOS app and presses Cmd + Shift + T
```

Flow:

1. App captures selected text.
2. App determines target language using primary/secondary rules.
3. App sends translation request to configured provider/model.
4. The panel opens near the tray, prefilled with the source text, and shows the translation.
5. User presses Enter to copy result and dismiss the panel.
6. User can also click Copy.

Failure states:

- No text selected.
- Selection capture failed.
- Provider API key missing.
- Provider returned error.
- Network unavailable.
- Required macOS permission missing.
- Shortcut conflict.

Each failure should show a compact actionable message.

### 10.3 Translate selected text by secondary hotkey

Trigger:

```text
User selects text in any macOS app and presses Cmd + Shift + Option + T
```

Flow:

1. App captures selected text.
2. App translates to configured secondary language.
3. The panel opens near the tray, prefilled with the source text, and shows the translation.
4. User presses Enter to copy result and dismiss the panel.

If secondary language is missing:

- App opens settings to language section.

### 10.4 Manual translation in the panel

Trigger:

```text
User clicks menu-bar icon
or
User presses manual translation shortcut
```

Default manual translation shortcut:

```text
Cmd + Option + T
```

Flow:

1. The panel drops down under the tray icon (see §19.2).
2. User types or pastes text into the input field.
3. User presses Enter or clicks Translate.
4. Translation appears below in a scrollable output field.
5. Copy button appears below the result.
6. Label says `Press Enter to copy` after translation completes.
7. User presses Enter.
8. App copies translation to clipboard and dismisses the panel.

### 10.5 Panel contents

TLiquid is a single window — a menu-bar panel (§19.2). Settings is a *view inside this panel*, reached by the gear icon, not a separate window. Quit lives on the tray's right-click menu, not inside the panel.

Phase 0 panel should include:

```text
┌ TLiquid ───────────────────── ⚙ ┐   ⚙ = open Settings view
│ Input text area                  │
│ Target: Auto / Primary /         │
│         Secondary / Choose lang   │
│ Translate button                 │
│ Output area                      │
│ Copy button                      │
└──────────────────────────────────┘
```

Language rows should not be top-level items in the tray menu. They live inside the Settings view or as a compact target-language selector in the panel.

### 10.6 Settings

Settings sections:

1. Languages
2. Shortcuts
3. Providers
4. Models
5. Output behavior
6. Privacy/diagnostics
7. Updates
8. About

#### 10.6.1 Languages section

User can:

- Set primary language.
- Set secondary language.
- Add unlimited additional languages.
- Remove additional languages.
- Reorder languages.
- Switch which language is primary.
- Switch which language is secondary.

Rules:

- Primary language is mandatory.
- English is the default primary language.
- Secondary language is optional.
- Additional languages are unlimited in local BYOK mode.

#### 10.6.2 Shortcuts section

Phase 0:

- Show default shortcuts.
- Allow enabling/disabling shortcuts if feasible.

Phase 1:

- Allow custom shortcut per action.
- Allow custom shortcut per additional language.
- Detect conflicts.

#### 10.6.3 Providers section

Supported Phase 0 providers:

- OpenAI
- Anthropic
- Gemini
- OpenRouter

Supported Phase 1 providers:

- Ollama/local models

For each provider:

- API key field where applicable.
- Save button.
- Test connection button.
- Status label:
  - Not configured
  - Configured
  - Invalid key
  - Connection failed

Security requirement:

- API keys should be stored in macOS Keychain in Phase 0.
- Plaintext settings file storage should be avoided by default.
- If plaintext key storage is ever supported, it must be explicit and marked as insecure.

#### 10.6.4 Models section

User can:

- Select default provider.
- Select default model.
- See disabled models for providers without configured keys.

Rules:

- If the key is absent for provider, models for that provider are inactive.
- If selected model becomes unavailable, app prompts user to choose another model.
- OpenRouter can be used as a convenience provider for many model families through one key.

#### 10.6.5 Output behavior

Phase 0:

```text
Selected-text translation output:
[x] Show result in panel
```

Phase 1:

```text
Selected-text translation output:
[ ] Show result in panel
[ ] Copy result to clipboard automatically
[ ] Replace selected text, where supported
```

Default:

```text
Show result in panel
```

Replacing selected text is not Phase 0 because it is risky and platform-sensitive.

#### 10.6.6 Privacy/diagnostics

Phase 0:

- No telemetry network calls.
- No diagnostics upload.
- Optional local diagnostics export may be added for bug reports.

Phase 2:

Optional setting:

```text
[ ] Send anonymous diagnostics to improve TLiquid
```

Default:

```text
OFF
```

Allowed diagnostic data if enabled:

- App version.
- OS name/version.
- Architecture.
- Crash stack traces.
- Startup time.
- Approximate memory usage.
- Provider type used, not API key.
- Error category, not text content.
- Feature usage counters, not user content.

Forbidden diagnostic data:

- Translation text.
- Clipboard contents.
- API keys.
- Provider request bodies.
- Provider responses.
- User file paths unless explicitly attached by the user in a bug report.

#### 10.6.7 Updates section

Phase 0:

- No automatic update checks.
- Show current version.
- Link to releases page.

Phase 2:

- Check on startup.
- Check once per day.
- Show `No updates available` label.
- Show `Update available` label.
- Provide `Update now` button.
- Download, install, and relaunch after user action.

---

## 11. Right-click integration plan

Right-click integration is not part of Phase 0.

Reason:

- It is platform-specific.
- It risks delaying the core MVP.
- Global hotkey translation is the more reliable initial path.

Planned phases:

| Platform | Phase | Approach |
|---|---:|---|
| macOS | Phase 1 | macOS Services / native integration |
| Windows | Phase 2 | Shell extension or supported context integration |
| Linux | Phase 3+ | Desktop-environment-specific best effort |

Guaranteed Phase 0 core flow remains:

```text
Select text → global hotkey → panel (prefilled)
```

---

## 12. Functional requirements

### 12.1 App shell

**FR-001:** Phase 0 app must run on macOS.  
**FR-002:** App must be open source from Phase 0.  
**FR-003:** App must use Rust + Tauri + Svelte unless a spike proves this stack unsuitable.  
**FR-004:** App must run as a single instance.  
**FR-005:** App must launch into background/menu-bar mode after setup.  
**FR-006:** App must expose a persistent macOS menu-bar icon.  
**FR-007:** App must not occupy Dock space while idle where technically possible.  
**FR-008:** App must allow user to quit from menu-bar menu.  
**FR-009:** Windows is not a verified Phase 0 target.  
**FR-010:** Linux is not a verified Phase 0 target.

### 12.2 Translation

**FR-011:** User must be able to translate manually from the panel input box.  
**FR-012:** User must be able to translate selected text using primary global shortcut on macOS.  
**FR-013:** User must be able to translate selected text using secondary global shortcut on macOS when secondary language exists.  
**FR-014:** App must auto-detect source language through provider prompt/model behavior.  
**FR-015:** App must determine target language using primary/secondary rules.  
**FR-016:** User must be able to copy translation result with a button.  
**FR-017:** User must be able to copy translation result by pressing Enter after translation completes.  
**FR-018:** App must show provider/network/permission errors in compact actionable form.  
**FR-019:** App must not store translation text by default.  
**FR-020:** App must not send translation text to TLiquid servers in Phase 0.

### 12.3 Languages

**FR-021:** User must have one mandatory primary language.  
**FR-022:** Default primary language must be English.  
**FR-023:** User may configure one secondary language.  
**FR-024:** User may configure unlimited additional target languages in local BYOK mode.  
**FR-025:** User must be able to switch primary language.  
**FR-026:** User must be able to switch secondary language.  
**FR-027:** User must be able to add, remove, and reorder additional languages.

### 12.4 Shortcuts

**FR-028:** App must register global shortcut for primary translation on macOS.  
**FR-029:** App must register global shortcut for secondary translation on macOS where secondary language exists.  
**FR-030:** App must register global shortcut for opening the translation panel on macOS.  
**FR-031:** App must show configured shortcuts in UI.  
**FR-032:** Phase 1 must allow user to change shortcuts.  
**FR-033:** App must detect shortcut registration failure and show user-readable error.  
**FR-034:** App must allow disabling shortcuts where feasible.

### 12.5 Providers and models

**FR-035:** User must be able to configure OpenAI API key.  
**FR-036:** User must be able to configure Anthropic API key.  
**FR-037:** User must be able to configure Gemini API key.  
**FR-038:** User must be able to configure OpenRouter API key.  
**FR-039:** Phase 1 must support Ollama/local model configuration.  
**FR-040:** User must be able to test provider connection.  
**FR-041:** User must be able to select default provider/model from active providers.  
**FR-042:** Models for providers without keys must be disabled.  
**FR-043:** App must expose provider abstraction so new providers can be added without rewriting UI flows.  
**FR-044:** App must support direct BYOK calls from user machine to provider in Phase 0.  
**FR-045:** App must support hosted proxy routing in Phase 2.

### 12.6 Settings

**FR-046:** User must be able to configure settings through UI.  
**FR-047:** User must be able to edit non-secret advanced settings through local settings file.  
**FR-048:** Settings UI must show location of config file.  
**FR-049:** Settings must persist across restarts.  
**FR-050:** Secrets should be stored in macOS Keychain in Phase 0.  
**FR-051:** App must not log API keys.  
**FR-052:** App must not expose API keys in crash reports or diagnostics.

### 12.7 Startup behavior

**FR-053:** Phase 1 must allow app to run on device startup.  
**FR-054:** Startup on login should be ON by default after user consent/onboarding.  
**FR-055:** User must be able to disable startup from settings.

### 12.8 Updates

**FR-056:** Phase 0 must not perform automatic update checks.  
**FR-057:** Phase 0 may show current version and releases link.  
**FR-058:** Phase 2 must check for updates once per day.  
**FR-059:** Phase 2 must check for updates on startup.  
**FR-060:** Phase 2 must show `No updates available` when current.  
**FR-061:** Phase 2 must show `Update available` when newer version exists.  
**FR-062:** Phase 2 must provide `Update now` button.  
**FR-063:** Phase 2 must download, install, and relaunch into fresh version after user confirms update.

### 12.9 Diagnostics

**FR-064:** Phase 0 must not send diagnostics to TLiquid servers.  
**FR-065:** Phase 0 may include local diagnostics export for user-submitted bug reports.  
**FR-066:** Phase 2 may add anonymous diagnostics only as explicit opt-in.  
**FR-067:** Diagnostics must never include translation text, clipboard contents, API keys, provider request bodies, or provider responses.

### 12.10 Monetization

**FR-068:** Local BYOK mode must support unlimited configured languages.  
**FR-069:** Local BYOK functionality should not be artificially restricted by language count.  
**FR-070:** Paid monetization should focus on hosted LLM proxy, managed usage, cloud sync, and convenience features.  
**FR-071:** Hosted proxy must meter usage and enforce plan limits.  
**FR-072:** Hosted proxy must not expose TLiquid provider API keys to clients.

### 12.11 Packaging and distribution

**FR-073:** Phase 0 must produce an installable macOS build.  
**FR-074:** Phase 0 must include direct distribution instructions.  
**FR-075:** Phase 0 should include signed/notarized macOS build if feasible, but unsigned local install is acceptable for internal MVP testing.  
**FR-076:** Windows packaging is not required in Phase 0.  
**FR-077:** Linux packaging is not required in Phase 0.

---

## 13. Non-functional requirements

### 13.1 Performance targets

Phase 0 macOS target metrics:

| Metric | Target |
|---|---:|
| Idle CPU | ~0% most of the time |
| Idle memory | As low as feasible; target under 80 MB |
| Disk footprint | As small as feasible; target under 30 MB compressed installer where realistic |
| Popup open latency | Under 150 ms after hotkey on supported macOS hardware |
| Manual translation UI responsiveness | No visible UI jank |
| Cold launch to menu-bar ready | Under 1 second on modern Mac |
| Translation latency | Mostly provider/network dependent |

These are product targets, not guaranteed framework-level claims.

### 13.2 Resource usage philosophy

TLiquid is an always-running utility. Therefore:

1. Avoid Electron.
2. Avoid heavy background workers.
3. Avoid polling loops.
4. Avoid unnecessary network calls.
5. Use a single window (the panel), kept hidden when idle. It is created once at startup so summoning it is an instant show/hide, not a repeated webview load — one webview's footprint rather than several.
6. Keep only menu-bar, shortcut listeners, and minimal app state active.

### 13.3 Reliability

1. App should recover if provider request fails.
2. App should not crash if selected-text capture fails.
3. App should keep menu-bar process alive unless user quits.
4. App should handle missing/invalid settings gracefully.
5. App should handle provider model deprecation by prompting user or falling back safely.
6. App should preserve clipboard content when using simulated copy mechanisms, where possible.

### 13.4 Privacy

1. Phase 0 must not send user text to TLiquid servers.
2. BYOK translation calls go directly to selected LLM provider.
3. App must clearly disclose that translated text is sent to the selected provider.
4. Translation history is off and absent in Phase 0.
5. No telemetry in Phase 0.
6. Future diagnostics must be opt-in.
7. API keys must be stored securely.

### 13.5 Security

1. Never log API keys.
2. Never include API keys in error messages.
3. Minimize macOS permissions.
4. Explain Accessibility/Automation permissions when needed.
5. Sign releases where possible.
6. Validate auto-update signatures in Phase 2.
7. Hosted proxy in Phase 2 must authenticate users and rate-limit requests.
8. Hosted proxy must protect TLiquid provider credentials.

### 13.6 Accessibility

1. Full keyboard operation for core flow.
2. Sufficient contrast in light/dark mode.
3. Screen-reader labels for settings fields and buttons.
4. Do not rely only on color for status.
5. Respect macOS reduced motion settings.

### 13.7 Internationalization

1. UI should support localization later.
2. Language list should use both native language name and English name where useful.
3. Prompt templates should be provider-neutral and language-aware.

---

## 14. Platform-specific behavior

### 14.1 macOS: Phase 0 verified platform

Expected behavior:

- App appears in macOS menu bar.
- App can run as menu-bar/background utility.
- Dock icon should be hidden when idle if feasible.
- Global shortcuts should work after registration.
- Selected-text capture may require Accessibility/Automation permission if implemented via simulated copy.
- Right-click integration is Phase 1 through macOS Services or native equivalent.
- Full Liquid Glass fidelity is not a Phase 0 requirement.

### 14.2 Windows: Phase 2 verified platform

Windows behavior is intentionally outside Phase 0.

Expected future behavior:

- App appears in system tray.
- No taskbar presence when no window is open.
- Global shortcuts should work if not conflicting.
- Selected-text capture may rely on clipboard simulation or UI Automation.
- Right-click integration is Phase 2 and may require shell extension work.

### 14.3 Linux: Phase 3+ verified platform

Linux behavior is intentionally outside Phase 0.

Expected future behavior:

- Tray/status icon support depends on desktop environment and tray protocol support.
- Global shortcuts depend on compositor/desktop environment.
- Wayland may impose restrictions that differ from X11.
- Selected-text capture may be inconsistent across environments.
- Right-click integration is Phase 3+ best effort and desktop-environment-specific.

---

## 15. Technical architecture

### 15.1 Recommended stack

```text
Desktop shell:      Tauri v2
Core language:      Rust
UI layer:           Svelte
Phase 0 platform:   macOS
State/config:       Local config file + macOS Keychain for secrets
Providers:          OpenAI, Anthropic, Gemini, OpenRouter
Future providers:   Ollama/local models
Distribution:       GitHub Releases + project website/direct download
Updates:            Manual in Phase 0, Tauri updater or equivalent in Phase 2
```

### 15.2 Why Svelte

Tauri provides the native shell, Rust backend, menu-bar integration, native windows, and system APIs.

The UI itself still needs a frontend layer because Tauri renders app UI through a WebView.

Svelte is selected because TLiquid needs a small, simple, reactive UI rather than a large web application framework.

Svelte should be used for the single panel and its views:

- Translate view (input + result).
- Settings view (reached by the gear icon, not a separate window).
- Provider/model forms.
- Language configuration.

These are views within one window, switched in the frontend — not separate native windows (§19.2).

If a technical spike shows that vanilla HTML/CSS/TypeScript is sufficient and smaller, that can be reconsidered. Default decision remains Svelte.

### 15.3 High-level components

```text
TLiquid App Process
├─ macOS Menu Bar Manager
├─ Global Shortcut Manager
├─ macOS Selection Capture Service
├─ Translation Orchestrator
├─ Language Routing Engine
├─ Provider Adapters
│  ├─ OpenAI Adapter
│  ├─ Anthropic Adapter
│  ├─ Gemini Adapter
│  ├─ OpenRouter Adapter
│  └─ Ollama Adapter, Phase 1
├─ Settings Manager
├─ macOS Keychain Secret Storage Manager
├─ Local Diagnostics Export Manager
├─ Update Manager, Phase 2
└─ UI: single menu-bar Panel (one window)
   ├─ Translate view (input + result)
   └─ Settings view (gear icon; not a separate window)
```

### 15.4 Language routing engine

Inputs:

```text
sourceText
selectedAction: primary | secondary | explicitLanguage
primaryLanguage
secondaryLanguage optional
explicitTargetLanguage optional
```

Output:

```text
targetLanguage
fallbackBehavior
promptInstructions
```

Routing rules:

```text
If selectedAction = explicitLanguage:
    target = explicitTargetLanguage

If selectedAction = secondary:
    target = secondaryLanguage

If selectedAction = primary:
    Ask model to detect source language.
    If detected source != primary:
        target = primary
    Else:
        target = secondary, if configured
```

Because model-based language detection happens inside the LLM call, the app can encode the conditional behavior directly into the prompt rather than making a separate detection call.

### 15.5 Provider abstraction

Provider interface:

```text
Provider
├─ id
├─ displayName
├─ validateKey(apiKey)
├─ listModels(apiKey)
├─ translate(request)
└─ supportsStreaming
```

Translation request:

```text
TranslationRequest
├─ sourceText
├─ targetLanguage
├─ primaryLanguage
├─ secondaryLanguage optional
├─ routingMode
├─ provider
├─ model
├─ preserveFormatting boolean
└─ outputMode
```

Translation response:

```text
TranslationResponse
├─ translatedText
├─ detectedSourceLanguage optional
├─ targetLanguage
├─ provider
├─ model
├─ latencyMs
├─ tokenUsage optional
└─ error optional
```

Transport and streaming (Phase 0 decisions):

- Adapters make **direct HTTP requests** to each provider's REST API using `reqwest` — **not** provider SDKs. There are no first-party Rust SDKs for OpenAI/Anthropic/Gemini/OpenRouter; TLiquid uses only a thin slice of each API (one translation call, optional model list, optional key check), and this `Provider` interface is already the abstraction, so wrapping several community SDKs behind it would add dependencies and divergent idioms for no gain. `reqwest` uses the system TLS stack (macOS Secure Transport), avoiding an OpenSSL build dependency and keeping the always-running utility small.
- **Phase 0 is non-streaming.** `translate` awaits the complete provider response and returns a single `TranslationResponse`. `supportsStreaming` exists on the interface but is `false` for every Phase 0 adapter.
- **Streaming is a Phase 1 goal** (§3.2): stream provider deltas — SSE for the cloud providers (OpenAI/Anthropic/Gemini/OpenRouter), NDJSON for Ollama — into the panel incrementally via a Tauri channel, and set `supportsStreaming` true per capable adapter. The non-streaming path remains a fallback.

### 15.6 Default prompt template

Primary-mode prompt:

```text
You are a translation engine.

Primary language: {primary_language}
Secondary language: {secondary_language_or_none}

Detect the source language of the text.

Rules:
1. If the source language is not the primary language, translate the text into the primary language.
2. If the source language is the primary language and a secondary language is configured, translate the text into the secondary language.
3. If the source language is the primary language and no secondary language is configured, translate into {fallback_target_language}.
4. Preserve meaning, tone, formatting, punctuation, markdown, code blocks, and technical terminology.
5. Return only the translation. Do not explain.

Text:
{text}
```

Explicit-target prompt:

```text
You are a translation engine.

Detect the source language automatically.
Translate the text into {target_language}.
Preserve meaning, tone, formatting, punctuation, markdown, code blocks, and technical terminology.
Return only the translation. Do not explain.

Text:
{text}
```

---

## 16. Settings model

Example non-secret settings file:

```json
{
  "version": 1,
  "startup": {
    "enabled": false
  },
  "ui": {
    "theme": "system",
    "accentColor": "default",
    "openResultFrom": "menu_bar"
  },
  "languages": {
    "primary": {
      "code": "en",
      "name": "English"
    },
    "secondary": {
      "code": "es",
      "name": "Spanish"
    },
    "additional": [
      {
        "code": "ru",
        "name": "Russian",
        "enabled": true
      },
      {
        "code": "de",
        "name": "German",
        "enabled": true
      }
    ]
  },
  "shortcuts": {
    "translatePrimary": "Cmd+Shift+T",
    "translateSecondary": "Cmd+Shift+Option+T",
    "openManualPopup": "Cmd+Option+T"
  },
  "providers": {
    "openai": {
      "enabled": true,
      "defaultModel": "gpt-4.1-mini"
    },
    "anthropic": {
      "enabled": false,
      "defaultModel": null
    },
    "gemini": {
      "enabled": false,
      "defaultModel": null
    },
    "openrouter": {
      "enabled": false,
      "defaultModel": null
    },
    "ollama": {
      "enabled": false,
      "defaultModel": null
    }
  },
  "defaultProvider": "openai",
  "defaultModel": "gpt-4.1-mini",
  "output": {
    "selectedTextBehavior": "show_popup",
    "copyOnEnter": true,
    "replaceSelection": false
  },
  "history": {
    "enabled": false
  },
  "diagnostics": {
    "enabled": false
  }
}
```

Secret values should not be stored here by default. Store provider keys in macOS Keychain in Phase 0 and reference provider IDs in config.

---

## 17. Monetization strategy

### 17.1 Open-source local mode

Local mode includes:

- Full app source code.
- Unlimited target languages.
- BYOK provider calls.
- Manual translation panel.
- Selected-text hotkey translation.
- Primary/secondary language routing.
- Provider/model configuration.

No artificial local language limit.

Rationale:

1. Local limits are weak in open-source software.
2. Artificial restrictions damage trust with developer/hacker/Linux users.
3. The strongest monetization path is hosted convenience, not local lockouts.

### 17.2 Paid cloud mode, Phase 2+

Paid cloud value may include:

1. Hosted LLM proxy.
2. No need to configure provider keys.
3. Usage bundles.
4. Cloud profiles.
5. Settings sync.
6. Translation history sync.
7. Translation memory.
8. Team/organization features.
9. Managed model presets.
10. Faster onboarding for non-technical users.

### 17.3 Hosted proxy behavior

When user chooses hosted proxy:

1. Client authenticates with TLiquid backend.
2. Client sends source text and target language to TLiquid proxy.
3. Proxy calls selected LLM provider using TLiquid provider credentials.
4. Proxy returns translation.
5. Proxy records minimal usage metadata for billing/rate limits.

Privacy requirement:

- Hosted mode must clearly disclose that text passes through TLiquid servers.
- Do not store full translation text by default.
- Store minimal metadata: user ID, timestamp, provider/model, token count, latency, status.

---

## 18. Update strategy

### 18.1 Phase 0

- Manual downloads via GitHub Releases or project website.
- App shows current version.
- App links to releases page.
- No automatic update checks.

### 18.2 Phase 2

Update behavior:

1. Check on startup.
2. Check once per day.
3. Show update state in settings.
4. If update available, show `Update now`.
5. User clicks update.
6. App downloads signed update.
7. App installs update.
8. App relaunches.
9. App returns to menu-bar state.

Update requirements:

- Signed update packages.
- Signature validation.
- Failure handling with safe retry.
- No forced silent update.

---

## 19. UI design requirements

### 19.1 Visual style

1. Minimalistic.
2. Small surface area.
3. System-like typography and spacing.
4. Light/dark mode follows macOS.
5. Accent color applied mainly to primary action button.
6. Avoid heavy custom UI unless necessary.
7. Avoid making the app feel like a full chat client.

### 19.2 Panel behavior (single-window decision)

TLiquid is **one window**: a frameless panel that drops down from the menu bar,
anchored near the tray icon. Everything (manual translation, the selected-text
result, and Settings) happens in this one panel; the frontend switches between
a translate view and a Settings view. There is no separate settings window and
no separate result popup.

Reference apps for the intended feel: **Raycast**, **Docker Desktop's tray
panel**, and **JetBrains Toolbox** — click the menu-bar icon (or press a
hotkey) and a compact panel appears at the top of the screen near the tray,
then dismisses when you're done.

Phase 0 behavior:

- The panel is a single window, created **once at startup and kept hidden**, so
  opening it is an instant show rather than a fresh window/webview load. This
  also keeps the idle footprint to one webview.
- Left-clicking the tray icon **toggles** the panel; it is positioned just
  below the clicked icon (a tray right-click opens a small Open/Settings/Quit menu).
- The panel **floats above other apps, including macOS fullscreen Spaces**, so
  it can be summoned from anywhere — like Docker/Raycast. Technically this is:
  Accessory activation policy (no Dock) + an always-on-top, visible-on-all-workspaces, frameless window.
- A selected-text hotkey opens this same panel **prefilled** with the source
  text and its translation (§10.2, §10.3) — not a separate overlay window.
- Cursor-positioned (follow-the-text-cursor) placement is **not** required;
  tray-anchored placement is the Phase 0 target.

Rationale (performance + UX): one window means one webview to keep warm, instant
view switching (a state change, not a window open), shared in-memory state
between the translate and Settings views, and no stray windows for the user to
manage. See `src-tauri/src/windows.rs`.

Future:

- Auto-hide the panel when it loses focus (Docker/Raycast-style dismissal).
- Cursor-positioned placement may be evaluated after multi-monitor, DPI, and OS
  permission issues are better understood.

### 19.3 macOS Liquid Glass consideration

Full macOS Liquid Glass fidelity is not a Phase 0 requirement.

Phase 0 approach:

- Use Tauri window transparency/blur where feasible.
- Keep design modern but simple.
- Prioritize reliability and small footprint over native visual effects.

Future approach:

- Consider macOS-specific native plugin only if visual fidelity becomes strategically important.

---

## 20. Key technical risks

### 20.1 macOS selected-text capture reliability

Capturing selected text from arbitrary macOS applications is not uniformly standardized.

Possible approaches:

1. Simulate copy shortcut, read clipboard, restore previous clipboard.
2. Use macOS Accessibility/Automation APIs.
3. Use platform-specific automation APIs.

Risks:

- Requires permissions.
- May briefly modify clipboard.
- Some apps block automation.
- Clipboard restoration may fail in edge cases.

Mitigation:

- Make hotkey workflow primary.
- Explain permissions clearly.
- Restore clipboard carefully.
- Provide manual panel fallback.
- Add local diagnostics export for capture issues.

### 20.2 Right-click context menu integration

Right-click integration is platform-specific and deferred.

Mitigation:

- Phase 0: no right-click integration.
- Phase 1: macOS only.
- Phase 2: Windows.
- Phase 3+: Linux best effort.

### 20.3 Windows/Linux deferred testing

Tauri keeps future portability possible, but unverified platform behavior can diverge.

Mitigation:

- Avoid hard-coding macOS assumptions into core business logic.
- Keep OS-specific code behind platform adapters.
- Do not claim Windows/Linux support until explicitly tested.

### 20.4 Shortcut conflicts

Global shortcuts can fail if another app owns them.

Mitigation:

- Detect registration failure.
- Show conflict warning.
- Allow remapping in Phase 1.

### 20.5 Provider model changes

LLM model names and availability change.

Mitigation:

- Fetch model list where provider supports it.
- Maintain fallback model list.
- Show provider/model errors clearly.

### 20.6 Open-source monetization

Since TLiquid is open source and local mode is unrestricted, monetization must rely on services rather than local feature lockouts.

Mitigation:

- Monetize hosted proxy.
- Monetize cloud profiles/sync/history/team features.
- Offer official signed builds and simple installers.
- Build trust through transparent local mode.

---

## 21. Success metrics

### 21.1 Phase 0 product activation

Since Phase 0 has no telemetry network calls, activation metrics can only come from voluntary channels:

- GitHub stars.
- GitHub issues.
- GitHub discussions.
- Release downloads.
- Website download counts if website exists.
- User-submitted diagnostics exports.

### 21.2 Phase 2 opt-in metrics

If anonymous diagnostics are added later, allowed metrics include:

- App version distribution.
- OS distribution.
- Crash rate.
- Startup time.
- Feature usage counters.
- Provider type distribution.
- Error category frequency.
- Selected-text capture failure rate.

Forbidden metrics:

- Translation content.
- Clipboard content.
- API keys.
- Full prompts.
- Full provider responses.

### 21.3 Product success indicators

- User can install and run Phase 0 app on macOS.
- User can configure at least one provider key.
- User can complete first manual translation.
- User can complete first selected-text translation.
- User keeps app running in menu bar.
- Low issue volume around resource usage.
- Positive feedback from developer/privacy-focused users.

---

## 22. MVP acceptance criteria

Phase 0 can be considered complete when:

1. App builds and runs on macOS.
2. App is open source with documented build instructions.
3. App uses Rust + Tauri + Svelte.
4. App starts and stays in menu-bar mode.
5. App does not show a Dock item while idle where macOS allows this.
6. User can open the translation panel from the tray icon or a hotkey.
7. User can configure at least one provider key.
8. User can select default model.
9. User has mandatory primary language, English by default.
10. User can configure optional secondary language.
11. User can configure unlimited additional languages.
12. User can translate manually with a real LLM provider.
13. User can translate selected text by primary global hotkey on macOS.
14. User can translate selected text by secondary global hotkey on macOS if secondary language is configured.
15. App shows meaningful error when selected-text capture fails.
16. API keys are not stored in plaintext by default.
17. Translation text is not sent to TLiquid servers.
18. No telemetry network calls occur in Phase 0.
19. Right-click integration is not required for Phase 0.
20. Basic documentation explains privacy, provider usage, permissions, limitations, and macOS-only Phase 0 scope.
21. Phase 0 produces an installable macOS artifact suitable for manual local installation.
22. Windows and Linux are not part of Phase 0 acceptance.

---

## 23. Suggested implementation milestones

### Milestone 0: macOS technical spike

Goal: prove the critical macOS integrations.

Deliverables:

1. Tauri app starts hidden in macOS menu bar.
2. Svelte panel opens from the menu bar, anchored near the tray icon.
3. Global shortcut opens the panel.
4. Selected-text capture works on macOS via simulated copy or accessibility path.
5. Basic OpenAI/Gemini/Anthropic/OpenRouter call works.
6. Clipboard restoration prototype.
7. macOS permission behavior documented.

### Milestone 1: Phase 0 MVP shell

Deliverables:

1. macOS menu-bar menu + tray-anchored panel.
2. Manual translation in the panel.
3. Settings view inside the panel (gear icon).
4. Language settings: primary, secondary, additional.
5. Provider API key configuration.
6. Model dropdown.
7. Local config persistence.
8. macOS Keychain secure key storage.

### Milestone 2: Phase 0 selected-text translation

Deliverables:

1. Primary translation shortcut.
2. Secondary translation shortcut.
3. macOS selected text capture.
4. Primary/secondary language routing.
5. Selected-text result shown in the panel (prefilled).
6. Enter-to-copy behavior.
7. Error handling.
8. Permission onboarding.

### Milestone 3: Phase 0 release polish

Deliverables:

1. macOS packaging.
2. macOS signed/notarized build if feasible.
3. README.
4. Privacy documentation.
5. GitHub Releases distribution.
6. Project website or landing page if needed.
7. Manual install instructions.

### Milestone 4: Phase 1

Deliverables:

1. Startup-on-login setting.
2. Shortcut customization.
3. Shortcut conflict detection.
4. Output behavior configuration.
5. Ollama/local model support.
6. macOS right-click integration.
7. Improved macOS diagnostics.
8. Better onboarding.

### Milestone 5: Phase 2

Deliverables:

1. Windows verified support.
2. Account/license system.
3. Hosted proxy backend.
4. Usage metering.
5. Paid cloud mode.
6. Auto-update check.
7. Update-now install/relaunch flow.
8. Windows right-click integration.
9. Optional anonymous diagnostics.
10. Optional translation history.

### Milestone 6: Phase 3+

Deliverables:

1. Linux verified support.
2. Linux packaging.
3. Linux global shortcut and selection support.
4. Linux tray compatibility matrix.
5. Linux right-click best-effort integration.
6. Advanced history/memory/cloud features.

---

## 24. Open questions

1. ~~Should TLiquid support streaming translation output or wait for full response?~~ **Resolved:** Phase 0 **waits for the full response** — translations are short, and it keeps the adapters and IPC simple. Streaming is deferred to Phase 1 (§3.2, §15.5).
2. Should the panel auto-close after copy, after timeout, or only manually?
3. Should the app support “copy source + translation” formatting?
4. Should OpenRouter be recommended as the easiest provider during onboarding?
5. Which models should be suggested by default for low-cost translation?
6. Should local diagnostics export exist in Phase 0?
7. Should official builds be signed from first public release?
8. Should selected-text capture use clipboard simulation as the first implementation?
9. Should translation history be strictly local when introduced, or cloud-syncable?
10. Which macOS versions should Phase 0 officially support?

---

## 25. Recommended Phase 0 product decision

For Phase 0, TLiquid should optimize for the workflow that is most likely to be reliable on macOS:

```text
Select text → press primary or secondary hotkey → panel shows translation → Enter copies → panel dismisses
```

Right-click integration, Windows, and Linux should not block the MVP.

The Phase 0 technical strategy should be:

```text
Rust + Tauri core
Svelte UI
macOS menu-bar utility
macOS global shortcuts
Primary/secondary language routing
Provider adapters
macOS Keychain secure local key storage
Manual panel fallback
Best-effort macOS selected-text capture
No telemetry network calls
No hosted backend dependency
Installable macOS build
```

The monetization strategy should be:

```text
Open-source unrestricted local BYOK mode
Future paid hosted proxy and cloud convenience features
```


# TLiquid roadmap

A snapshot of where TLiquid is going. No dates, no commitments — this is the
*direction*, not a contract. The spec that grounds all of it lives in
[`llm_translator_prd.md`](./llm_translator_prd.md).

For shipped functionality see the GitHub [Releases](https://github.com/cesarion161/TLiquid/releases)
page.

## Now

What's live in the current release and being polished.

- **macOS menu-bar panel** — frameless, translucent, summon-and-forget.
  Spotlight-style anchor under the tray, draggable, position remembered.
- **BYOK to four cloud providers** — OpenAI, Anthropic, Gemini, OpenRouter.
  Streamed responses, direct from your machine, keys in the macOS Keychain.
- **Local Ollama** as a first-class keyless option for fully-offline use.
- **Global hotkeys** — translate the selection (`⌘⇧T` by default), translate
  to a secondary language, optional per-additional-language shortcuts.
  All user-configurable.
- **In-app updates** — minisign-verified, manual install (never silent), with
  an opt-out auto-check.
- **Landing page + custom domain** at [tliquid.app](https://tliquid.app).

## Next

The near-term focus, roughly in priority order.

- **Demo content + soft launch** — a real demo video on the site, then
  introducing TLiquid in a few low-stakes places (HN Show, r/macapps,
  Product Hunt).
- **Onboarding polish** — making first-run "install Ollama, pick a model, done"
  feel one-click for users without an API key.
- **Provider matrix expansion** — adding a couple of frequently-requested
  cloud providers and surfacing model picking more clearly.
- **macOS quality-of-life** — better error messaging, smoother capture on edge
  cases (PDF readers, Electron apps), accessibility pass.

## Later

Bigger product directions being weighed but not committed.

- **Verified Windows and Linux support** — both possible technically; both
  blocked on owning a real test machine for each platform and resolving
  platform-specific selection-capture stories.
- **Translation memory** — opt-in local cache of common phrases, fully
  private, never synced unless explicitly turned on.
- **Advanced translation modes** — literal / natural / professional / casual,
  and a "preserve markdown" mode for code-adjacent translation.
- **Optional hosted tier** — a thin pay-as-you-go alternative to BYOK for
  users who don't want to manage API keys. Strict design constraint: must not
  compromise the BYOK / local-Ollama paths that exist today.

## What this roadmap is not

- A commitment. Anything here can move, get cut, or get reordered without
  notice. The only guarantee is the released binaries and the
  [LICENSE](./LICENSE).
- A request for funding or staffing. TLiquid is a single-maintainer indie
  project.
- A complete list. Smaller polish, bug fixes, and refactors aren't tracked
  here.

Want something on it? Open a GitHub Issue with the use case. The
fastest-moving items are usually the ones with a clearly-described user
journey behind them.

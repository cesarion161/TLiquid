// Shared flag: true while ShortcutSettings is recording a new global shortcut
// (P1-002). Recording pauses the live global shortcuts so the combo reaches the
// webview (macOS consumes registered global hotkeys). All Settings sections
// share one scrollable view and persist through the same `Settings.persist()`,
// which re-registers shortcuts — so it must NOT re-register while recording, or
// a save from another section (Languages/Providers) would silently un-pause the
// shortcuts mid-recording. `Settings.persist()` checks this; ShortcutSettings
// sets it around a recording. (A plain module flag, read synchronously at
// persist time — no reactivity needed.)
let recording = false;

export const setRecordingShortcut = (value: boolean): void => {
  recording = value;
};

export const isRecordingShortcut = (): boolean => recording;

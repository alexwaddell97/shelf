// Small differences between Windows/Linux and macOS in what the interface says.

export const isMac = typeof navigator !== 'undefined' && /Mac/i.test(navigator.userAgent);

/** The modifier key as people know it: "⌘" on a Mac, "Ctrl" elsewhere. */
export const MOD = isMac ? '⌘' : 'Ctrl';

/** A shortcut for labels and tooltips: shortcut('K') is "⌘K" on a Mac, "Ctrl+K" elsewhere. */
export const shortcut = (key: string) => (isMac ? `⌘${key}` : `Ctrl+${key}`);

const PALETTE_KEY = 'k';

type ChordEvent = Pick<
	KeyboardEvent,
	'key' | 'ctrlKey' | 'metaKey' | 'altKey' | 'shiftKey'
>;

/** Ctrl+K or Meta+K, with no other modifier. */
export function isPaletteChord(event: Readonly<ChordEvent>): boolean {
	if (event.key.toLowerCase() !== PALETTE_KEY) return false;
	if (event.altKey || event.shiftKey) return false;
	return event.ctrlKey !== event.metaKey;
}

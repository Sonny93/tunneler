const FIRST_INDEX = 0;

/**
 * The index to focus for a navigation key, clamped (no wrap), or `null` when
 * the key is not a navigation key.
 */
export function resolveRovingIndex(
	key: string,
	currentIndex: number,
	count: number
): number | null {
	const lastIndex = count - 1;
	switch (key) {
		case 'ArrowDown':
			return Math.min(currentIndex + 1, lastIndex);
		case 'ArrowUp':
			return Math.max(currentIndex - 1, FIRST_INDEX);
		case 'Home':
			return FIRST_INDEX;
		case 'End':
			return lastIndex;
		default:
			return null;
	}
}

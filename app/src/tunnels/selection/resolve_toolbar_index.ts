const FIRST_INDEX = 0;

/**
 * The index to focus for a key of the ARIA toolbar pattern (←, →, Home, End;
 * the arrows wrap around), or `null` when the key is not a navigation key.
 */
export function resolveToolbarIndex(
	key: string,
	currentIndex: number,
	count: number
): number | null {
	const lastIndex = count - 1;
	switch (key) {
		case 'ArrowRight':
			return currentIndex >= lastIndex ? FIRST_INDEX : currentIndex + 1;
		case 'ArrowLeft':
			return currentIndex <= FIRST_INDEX ? lastIndex : currentIndex - 1;
		case 'Home':
			return FIRST_INDEX;
		case 'End':
			return lastIndex;
		default:
			return null;
	}
}

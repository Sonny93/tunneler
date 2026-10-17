export type ActiveIndexMove = 'next' | 'previous' | 'first' | 'last';

/** Where the active option goes; stepping wraps around, `null` when there are no options. */
export function moveActiveIndex(
	currentIndex: number,
	optionCount: number,
	move: ActiveIndexMove
): number | null {
	if (optionCount === 0) return null;
	switch (move) {
		case 'next':
			return (currentIndex + 1) % optionCount;
		case 'previous':
			return (currentIndex - 1 + optionCount) % optionCount;
		case 'first':
			return 0;
		case 'last':
			return optionCount - 1;
	}
}

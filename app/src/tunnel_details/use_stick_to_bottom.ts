import { useEffect, useRef, type RefObject } from 'react';

const BOTTOM_TOLERANCE_PIXELS = 8;

type UseStickToBottomReturn = {
	handleScroll: () => void;
};

/** Keeps a scroll container on its last item as items are added, unless the user scrolled up. */
export function useStickToBottom(
	containerRef: RefObject<HTMLElement | null>,
	itemCount: number
): UseStickToBottomReturn {
	const isStuckToBottom = useRef(true);

	useEffect(() => {
		const container = containerRef.current;
		if (container && isStuckToBottom.current) {
			container.scrollTop = container.scrollHeight;
		}
	}, [containerRef, itemCount]);

	const handleScroll = () => {
		const container = containerRef.current;
		if (!container) return;
		const distanceToBottom =
			container.scrollHeight - container.scrollTop - container.clientHeight;
		isStuckToBottom.current = distanceToBottom <= BOTTOM_TOLERANCE_PIXELS;
	};

	return { handleScroll };
}

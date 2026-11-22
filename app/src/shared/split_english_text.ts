export type TextSegment = { text: string; isEnglish: boolean };

type Match = { start: number; fragment: string };

function findEarliestMatch(
	text: string,
	fragments: readonly string[]
): Match | null {
	let earliest: Match | null = null;
	for (const fragment of fragments) {
		const start = text.indexOf(fragment);
		if (start === -1) continue;
		const isBetter =
			earliest === null ||
			start < earliest.start ||
			(start === earliest.start && fragment.length > earliest.fragment.length);
		if (isBetter) earliest = { start, fragment };
	}
	return earliest;
}

/**
 * Cuts `text` around the fragments that are raw English from the daemon, so
 * they can be marked `lang="en"` inside a translated sentence.
 */
export function splitEnglishText(
	text: string,
	fragments: readonly string[]
): TextSegment[] {
	const usable = fragments.filter((fragment) => fragment !== '');
	const segments: TextSegment[] = [];
	let rest = text;
	for (;;) {
		const match = findEarliestMatch(rest, usable);
		if (match === null) break;
		if (match.start > 0) {
			segments.push({ text: rest.slice(0, match.start), isEnglish: false });
		}
		segments.push({ text: match.fragment, isEnglish: true });
		rest = rest.slice(match.start + match.fragment.length);
	}
	if (rest !== '') segments.push({ text: rest, isEnglish: false });
	return segments;
}

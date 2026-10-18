/**
 * The ids from `anchorId` to `targetId`, both included, in the order of
 * `orderedIds`. Only `targetId` when the anchor is unknown or not listed.
 */
export function getRangeIds(
	orderedIds: readonly string[],
	anchorId: string | null,
	targetId: string
): readonly string[] {
	const anchorIndex = anchorId === null ? -1 : orderedIds.indexOf(anchorId);
	const targetIndex = orderedIds.indexOf(targetId);
	if (anchorIndex === -1 || targetIndex === -1) return [targetId];
	const start = Math.min(anchorIndex, targetIndex);
	const end = Math.max(anchorIndex, targetIndex);
	return orderedIds.slice(start, end + 1);
}

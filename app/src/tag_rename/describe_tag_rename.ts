import { formatTunnelCount } from '#/tunnels/selection/describe_selection_count';

/** « Tag dev renommé en staging (3 tunnels) ». */
export function describeTagRename(
	from: string,
	to: string,
	renamedCount: number
): string {
	return `Tag ${from} renommé en ${to} (${formatTunnelCount(renamedCount)})`;
}

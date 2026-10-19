import { formatTunnelCount } from '#/tunnels/selection/describe_selection_count';

/** « Le tag sera retiré de 3 tunnels. Les tunnels sont conservés. », plus the launch tags when the tag is one of them. */
export function describeTagDeletionWarning(
	tunnelCount: number,
	isLaunchTag: boolean
): string {
	const warning = `Le tag sera retiré de ${formatTunnelCount(tunnelCount)}. Les tunnels sont conservés.`;
	if (!isLaunchTag) return warning;
	return `${warning} Il sera aussi retiré des « Tags démarrés au lancement ».`;
}

/** « Tag dev retiré de 3 tunnels ». */
export function describeTagDeletion(tag: string, removedCount: number): string {
	return `Tag ${tag} retiré de ${formatTunnelCount(removedCount)}`;
}

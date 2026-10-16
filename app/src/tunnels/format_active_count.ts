import type { ActiveCount } from '#/tunnels/active_count';

/** Visual form, such as `2/5 actifs`. */
export function formatActiveCount({
	running,
	total,
}: Readonly<ActiveCount>): string {
	return `${running}/${total} actifs`;
}

import type { ActiveCount } from '#/tunnels/active_count';

/** Spoken form, such as `2 sur 5 actifs`. */
export function describeActiveCount({
	running,
	total,
}: Readonly<ActiveCount>): string {
	return `${running} sur ${total} actifs`;
}

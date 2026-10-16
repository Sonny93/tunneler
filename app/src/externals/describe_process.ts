import type { ProcessOwnerDto } from '#/daemon/protocol';

/** Example: `ssh (pid 4242)`. */
export function describeProcess({
	process,
	pid,
}: Readonly<ProcessOwnerDto>): string {
	return `${process} (pid ${pid})`;
}

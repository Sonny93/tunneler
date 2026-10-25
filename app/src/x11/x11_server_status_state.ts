/** What the last probe of the local X server found. */
export type X11ServerStatus =
	| { kind: 'checking' }
	| { kind: 'reachable'; display: string }
	| { kind: 'unreachable'; display: string; server: string | null }
	| { kind: 'failed'; reason: string };

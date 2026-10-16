type TunnelTagChipsProps = {
	tags: readonly string[];
};

export function TunnelTagChips({ tags }: Readonly<TunnelTagChipsProps>) {
	if (tags.length === 0) return null;

	return (
		<ul
			role="list"
			aria-label="Tags"
			className="m-0 flex list-none flex-wrap gap-1 p-0"
		>
			{tags.map((tag) => (
				<li
					key={tag}
					className="rounded bg-tn-surface-raised px-1.5 py-0.5 text-xs text-tn-muted"
				>
					{tag}
				</li>
			))}
		</ul>
	);
}

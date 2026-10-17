const LAUNCHER_SNIPPET = `[[x11_launchers]]
host = "nova"
name = "VS Code"
command = "code"`;

export const X11LaunchersEmpty = () => (
	<div className="flex flex-col gap-2 text-sm">
		<p className="m-0">
			Aucun lanceur pour cet hôte. Les lanceurs se déclarent dans{' '}
			<code className="font-mono">tunnels.toml</code> :
		</p>
		<pre className="m-0 overflow-x-auto rounded-md bg-tn-neutral-subtle p-2 font-mono text-xs">
			{LAUNCHER_SNIPPET}
		</pre>
	</div>
);

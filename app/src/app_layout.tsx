import type { ReactNode } from 'react';

const MAIN_ID = 'main-content';

type AppLayoutProps = {
	sidebar: ReactNode;
	actions: ReactNode;
	children: ReactNode;
};

export const AppLayout = ({
	sidebar,
	actions,
	children,
}: Readonly<AppLayoutProps>) => (
	<div className="flex min-h-screen flex-col bg-tn-bg text-tn-text">
		<a
			href={`#${MAIN_ID}`}
			className="sr-only focus:not-sr-only focus:absolute focus:left-2 focus:top-2 focus:z-50 focus:rounded-md focus:bg-tn-surface-raised focus:px-3 focus:py-2"
		>
			Aller à la liste des tunnels
		</a>
		<header className="flex flex-wrap items-center justify-between gap-2 border-b border-tn-divider px-4 py-2">
			<h1 className="text-lg font-semibold">Tunneler</h1>
			<div className="flex flex-wrap items-center gap-2">{actions}</div>
		</header>
		<div className="flex min-w-0 flex-1 flex-col md:flex-row">
			<aside
				aria-label="Navigation"
				className="border-b border-tn-divider bg-tn-surface p-4 md:w-64 md:shrink-0 md:border-b-0 md:border-r"
			>
				{sidebar}
			</aside>
			<main id={MAIN_ID} tabIndex={-1} className="min-w-0 flex-1 p-4">
				{children}
			</main>
		</div>
	</div>
);

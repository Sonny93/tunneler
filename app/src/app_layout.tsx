import { useLingui } from '@lingui/react/macro';
import type { ReactNode } from 'react';

const MAIN_ID = 'main-content';

type AppLayoutProps = {
	sidebar: ReactNode;
	sidebarToggle: ReactNode;
	isSidebarCollapsed: boolean;
	actions: ReactNode;
	children: ReactNode;
};

export function AppLayout({
	sidebar,
	sidebarToggle,
	isSidebarCollapsed,
	actions,
	children,
}: Readonly<AppLayoutProps>) {
	const { t } = useLingui();

	return (
		<div className="flex h-screen flex-col overflow-hidden bg-tn-bg text-tn-text">
			<a
				href={`#${MAIN_ID}`}
				className="sr-only focus:not-sr-only focus:absolute focus:left-2 focus:top-2 focus:z-50 focus:rounded-md focus:bg-tn-surface-raised focus:px-3 focus:py-2"
			>
				{t`Skip to the tunnel list`}
			</a>
			<header className="flex shrink-0 flex-wrap items-center justify-between gap-2 border-b border-tn-divider px-4 py-2">
				<div className="flex items-center gap-2">
					{sidebarToggle}
					<h1 className="text-lg font-semibold">Tunneler</h1>
				</div>
				<div className="flex flex-wrap items-center gap-2">{actions}</div>
			</header>
			<div className="flex min-h-0 min-w-0 flex-1 flex-col overflow-y-auto md:flex-row md:overflow-hidden">
				<aside
					aria-label={t`Navigation`}
					hidden={isSidebarCollapsed}
					className="border-b border-tn-divider bg-tn-surface p-4 md:min-h-0 md:w-64 md:shrink-0 md:overflow-y-auto md:border-b-0 md:border-r"
				>
					{sidebar}
				</aside>
				<main
					id={MAIN_ID}
					tabIndex={-1}
					className="min-w-0 flex-1 p-4 md:min-h-0 md:overflow-y-auto"
				>
					{children}
				</main>
			</div>
		</div>
	);
}

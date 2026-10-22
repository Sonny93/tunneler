import { ModalProvider } from '@minimalstuff/ui';

import { Overlays } from '#/overlays';
import { AppLayout } from '#/app_layout';
import { Sidebar } from '#/sidebar/sidebar';
import { useUiStore } from '#/state/ui_store';
import { ConnectingScreen } from '#/connecting_screen';
import { UpdateNotice } from '#/updates/update_notice';
import { HeaderActions } from '#/header/header_actions';
import { TunnelList } from '#/tunnels/list/tunnel_list';
import { SidebarToggle } from '#/sidebar/sidebar_toggle';
import { DisconnectedScreen } from '#/disconnected_screen';
import { ToastRegion } from '#/shared/toasts/toast_region';
import { useUpdateCheck } from '#/updates/use_update_check';
import { TrayStatusNotice } from '#/tray/tray_status_notice';
import { ExternalSection } from '#/externals/external_section';
import { LiveAnnouncer } from '#/shared/announcer/live_announcer';
import { useX11Announcements } from '#/x11/use_x11_announcements';
import { useDaemonConnection } from '#/state/use_daemon_connection';
import { useTunnelStateAnnouncements } from '#/tunnels/use_tunnel_state_announcements';

export function App() {
	const { connection, retry } = useDaemonConnection();
	const isSidebarCollapsed = useUiStore((state) => state.isSidebarCollapsed);
	useTunnelStateAnnouncements();
	useX11Announcements();
	useUpdateCheck();

	return (
		<>
			{connection.status === 'connecting' && <ConnectingScreen />}
			{connection.status === 'disconnected' && (
				<DisconnectedScreen message={connection.message} onRetry={retry} />
			)}
			{connection.status === 'connected' && (
				<>
					<AppLayout
						sidebar={<Sidebar />}
						sidebarToggle={<SidebarToggle />}
						isSidebarCollapsed={isSidebarCollapsed}
						actions={<HeaderActions />}
					>
						<TrayStatusNotice />
						<UpdateNotice />
						<TunnelList />
						<ExternalSection />
					</AppLayout>
					<Overlays />
				</>
			)}
			<LiveAnnouncer />
			<ToastRegion />
			<ModalProvider />
		</>
	);
}

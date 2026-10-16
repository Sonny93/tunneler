import { ModalProvider } from '@minimalstuff/ui';

import { Overlays } from '#/overlays';
import { AppLayout } from '#/app_layout';
import { Sidebar } from '#/sidebar/sidebar';
import { ConnectingScreen } from '#/connecting_screen';
import { HeaderActions } from '#/header/header_actions';
import { TunnelList } from '#/tunnels/list/tunnel_list';
import { DisconnectedScreen } from '#/disconnected_screen';
import { ToastRegion } from '#/shared/toasts/toast_region';
import { TrayStatusNotice } from '#/tray/tray_status_notice';
import { ExternalSection } from '#/externals/external_section';
import { LiveAnnouncer } from '#/shared/announcer/live_announcer';
import { useDaemonConnection } from '#/state/use_daemon_connection';
import { useTunnelStateAnnouncements } from '#/tunnels/use_tunnel_state_announcements';

export function App() {
	const { connection, retry } = useDaemonConnection();
	useTunnelStateAnnouncements();

	return (
		<>
			{connection.status === 'connecting' && <ConnectingScreen />}
			{connection.status === 'disconnected' && (
				<DisconnectedScreen message={connection.message} onRetry={retry} />
			)}
			{connection.status === 'connected' && (
				<>
					<AppLayout sidebar={<Sidebar />} actions={<HeaderActions />}>
						<TrayStatusNotice />
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

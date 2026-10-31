import NotFound from '@stevederico/skateboard-ui/NotFound';
import KioskView from './KioskView.tsx';

/** Paths that are the kiosk page besides `/` (Rails served `public/kiosk.html` there). */
const KIOSK_PATHS = new Set(['/kiosk.html', '/kiosk']);

/**
 * Shell `notFound` override: render the kiosk on its static-file paths, the
 * shell's NotFound everywhere else. The server already 404s unknown paths, so
 * this only matters for those two URLs.
 */
export default function KioskFallback() {
  return KIOSK_PATHS.has(window.location.pathname) ? <KioskView /> : <NotFound />;
}

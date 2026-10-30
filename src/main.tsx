/**
 * Kiosk entry point on the Skateboard application shell.
 *
 * The kiosk is the landing page at `/` (the Rails `root 'pages#index'`). It has
 * no login (`noLogin` in constants.json) and no `/app` routes.
 */
import './assets/styles.css';
import { createSkateboardApp } from '@stevederico/skateboard-ui/App';
import KioskFallback from './components/KioskFallback.tsx';
import KioskView from './components/KioskView.tsx';
import constants from './constants.json';

createSkateboardApp({
  constants,
  appRoutes: [],
  landingPage: <KioskView />,
  overrides: { notFound: KioskFallback },
  loadLegal: () => import('./legal.json')
});

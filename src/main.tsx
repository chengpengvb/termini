import App from './App';
import 'reflect-metadata';
import React from 'react';
import ReactDOM from 'react-dom/client';
import { FatalPage } from '@/page/fatal/FatalPage';
import { ServicesProvider } from '@/common/context/ServiceContext';
import { PlatformProvider } from '@/common/context/PlatformContext';
import { AppThemeProvider } from '@/common/context/AppThemeProvider';
import { installBrowserShortcutBlocker } from '@/common/util/BrowserShortcutUtil';
import { installCrashReport, installErrorReport } from '@/common/util/ErrorReportUtil';

void import('./App.css');
installBrowserShortcutBlocker();
installErrorReport();
installCrashReport();

document.documentElement.classList.add('dark');


ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
  <React.StrictMode>
    <AppThemeProvider>
      <FatalPage />
      <ServicesProvider>
        <PlatformProvider>
          <App />
        </PlatformProvider>
      </ServicesProvider>
    </AppThemeProvider>
  </React.StrictMode>,
);
void import('./styles/terminal/fonts.css');

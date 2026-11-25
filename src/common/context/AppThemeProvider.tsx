import {ThemeProvider, createTheme} from '@mui/material/styles';
import {useEffect, useMemo, useState, type ReactNode} from 'react';
import {MENU_TRANSITION_DURATION, MenuTransition} from '@/common/components/menu/MenuTransition';

export function AppThemeProvider(props: {children: ReactNode; appearance?: 'default' | 'light'}) {
  const [appLight, setAppLight] = useState(() => document.documentElement.getAttribute('data-app-theme') === 'light');
  const light = props.appearance ? props.appearance === 'light' : appLight;
  const theme = useMemo(() => createTheme({
  palette: {
    mode: light ? 'light' : 'dark',
    primary: {
      main: light ? '#2463b5' : '#408fd8',
      dark: light ? '#1c4f91' : '#66a4dc',
      contrastText: light ? '#ffffff' : '#101820',
    },
    background: {
      default: light ? '#f5f6f8' : '#181818',
      paper: light ? '#ffffff' : '#252525',
    },
    text: {
      primary: light ? '#20242b' : '#d4d4d4',
      secondary: light ? '#4f5b6b' : '#a1a1a1',
    },
    error: {
      main: light ? '#b02834' : '#f67a7a',
    },
  },
  typography: {
    fontFamily: "-apple-system, BlinkMacSystemFont, 'Segoe UI', 'PingFang SC', 'Microsoft YaHei', sans-serif",
  },
  shape: {
    borderRadius: 8,
  },
  transitions: {
    duration: {
      shortest: 120,
      shorter: 150,
      short: 200,
      standard: 250,
      complex: 320,
      enteringScreen: 200,
      leavingScreen: 150,
    },
    easing: {
      easeInOut: 'cubic-bezier(0.32, 0.72, 0, 1)',
      easeOut: 'cubic-bezier(0.32, 0.72, 0, 1)',
      easeIn: 'cubic-bezier(0.4, 0, 0.2, 1)',
      sharp: 'cubic-bezier(0.4, 0, 0.2, 1)',
    },
  },
  components: {
    MuiInputBase: {
      styleOverrides: {
        input: {
          '&::placeholder': {color: 'var(--tm-text-tertiary)', opacity: 1},
        },
      },
    },
    MuiMenu: {
      defaultProps: {
        TransitionComponent: MenuTransition,
        transitionDuration: MENU_TRANSITION_DURATION,
      },
    },
    MuiPopover: {
      defaultProps: {
        TransitionComponent: MenuTransition,
        transitionDuration: MENU_TRANSITION_DURATION,
      },
    },
  },
}), [light]);

  useEffect(() => {
    const root = document.documentElement;
    const sync = (): void => setAppLight(root.getAttribute('data-app-theme') === 'light');
    const observer = new MutationObserver(sync);
    observer.observe(root, {attributes: true, attributeFilter: ['data-app-theme']});
    sync();
    return () => observer.disconnect();
  }, []);

  return <ThemeProvider theme={theme}>{props.children}</ThemeProvider>;
}

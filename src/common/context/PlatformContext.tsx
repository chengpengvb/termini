import type { ReactNode } from 'react';
import { useEffect, useState } from 'react';
import { useMediaQuery } from '@mui/material';
import { PlatformContext } from '@/common/context/platformStore';
import { useServiceContainer } from '@/common/context/ServiceContext';
import { SystemUtilServiceImpl } from '@/service/impl/SystemUtilServiceImpl';

const RETRY_MS = 250;
const RETRY_MAX = 40;
const MOBILE_MQ = '(max-width: 899.95px)';

function syncMobileClass(mobile: boolean, nativeMobile: boolean): void {
  document.documentElement.classList.toggle('tm-platform-mobile', mobile);
  document.documentElement.classList.toggle('tm-native-mobile', nativeMobile);
}

syncMobileClass(window.matchMedia(MOBILE_MQ).matches, false);

export function PlatformProvider(props: { children: ReactNode }) {
  const container = useServiceContainer();
  const viewportMobile = useMediaQuery(MOBILE_MQ);
  const [platformMobile, setPlatformMobile] = useState(false);
  const mobile = platformMobile || viewportMobile;

  useEffect(() => {
    let timer = 0;
    let attempt = 0;
    let cancelled = false;
    const systemUtilService = container.resolve(SystemUtilServiceImpl);

    const load = () => {
      void systemUtilService
        .getPlatformInfo()
        .then(info => {
          if (!cancelled) {
            setPlatformMobile(info.mobile);
          }
        })
        .catch(() => {
          if (cancelled) {
            return;
          }
          attempt += 1;
          if (attempt >= RETRY_MAX) {
            setPlatformMobile(false);
            return;
          }
          timer = window.setTimeout(load, RETRY_MS);
        });
    };

    load();
    return () => {
      cancelled = true;
      window.clearTimeout(timer);
    };
  }, [container]);

  useEffect(() => {
    syncMobileClass(mobile, platformMobile);
  }, [mobile, platformMobile]);

  return <PlatformContext.Provider value={{ mobile, nativeMobile: platformMobile }}>{props.children}</PlatformContext.Provider>;
}

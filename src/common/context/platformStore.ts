import {createContext} from 'react';

export interface PlatformContextValue {
    mobile: boolean;
    nativeMobile: boolean;
}

export const PlatformContext = createContext<PlatformContextValue>({mobile: false, nativeMobile: false});

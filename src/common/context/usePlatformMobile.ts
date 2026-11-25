import {useContext} from 'react';
import {PlatformContext} from '@/common/context/platformStore';

export function usePlatformMobile(): boolean {
    return useContext(PlatformContext).mobile;
}

export function usePlatformNativeMobile(): boolean {
    return useContext(PlatformContext).nativeMobile;
}

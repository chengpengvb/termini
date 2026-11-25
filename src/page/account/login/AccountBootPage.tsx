import {Box} from '@mui/material';
import logoUrl from '@/assets/icon/logo.svg';
import {useSystemUtilService} from '@/common/context/useAppHooks.tsx';
import {BarControls} from '@/common/components/tab/components/tabBar/BarControls.tsx';

export default function AccountBootPage() {
    const systemUtilService = useSystemUtilService();

    return (
        <Box className="tm-account-login is-boot">
            <Box className="tm-account-login-titlebar">
                <Box className="tm-account-login-drag" onMouseDown={() => void systemUtilService.startWindowDragging()}/>
                <BarControls/>
            </Box>
            <Box component="img" className="tm-account-login-logo" src={logoUrl} alt="Termini"/>
        </Box>
    );
}

import {Box} from '@mui/material';
import {useI18n} from '@/common/i18n/useI18n.ts';
import {ConfigTermBase} from "@/common/model/ConfigTermBase.ts";
import { EditField } from '@/page/common/components/input/EditField.tsx';
import {parsePort, resolveDisplayNumber} from "@/page/common/util/numberInputUtil.ts";

export function BasicFields(props: { connection: ConfigTermBase; onChange: (connection: ConfigTermBase) => void; portPlaceholder?: string; usernamePlaceholder?: string }) {
    const {t} = useI18n();
    const { connection, onChange, portPlaceholder, usernamePlaceholder } = props;

    const handlePortChange = (raw: string): void => {
        onChange({ ...connection, port: parsePort(raw) });
    };

    const handleUsernameChange = (username: string): void => {
        onChange({ ...connection, username });
    };

    const handlePasswordChange = (password: string): void => {
        onChange({ ...connection, password });
    };

    return (
        <>
            <Box className="tm-device-edit-row">
                <EditField label={t('home.common.port')} value={resolveDisplayNumber(connection.port)} onChange={handlePortChange} placeholder={portPlaceholder} />
                <EditField label={t('home.common.username')} value={connection.username} onChange={handleUsernameChange} placeholder={usernamePlaceholder} />
            </Box>
            <EditField label={t('home.common.password')} value={connection.password} onChange={handlePasswordChange} placeholder={t('home.devices.passwordPlaceholder')} password />
        </>
    );
}

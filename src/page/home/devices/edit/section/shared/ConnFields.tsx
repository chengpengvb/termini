import {useI18n} from '@/common/i18n/useI18n.ts';
import {ConfigTermBase} from "@/common/model/ConfigTermBase.ts";
import { EditField } from '@/page/common/components/input/EditField.tsx';
import {parseNonNegativeInt, parseOptionalSeconds, resolveDisplayNumber, resolveOptionalNumber} from "@/page/common/util/numberInputUtil.ts";

export function ConnFields(props: { connection: ConfigTermBase; onChange: (connection: ConfigTermBase) => void }) {
    const {t} = useI18n();
    const { connection, onChange } = props;

    const handleConnectTimeoutChange = (raw: string): void => {
        onChange({ ...connection, connect_timeout_seconds: parseOptionalSeconds(raw) });
    };

    const handleKeepaliveIntervalChange = (raw: string): void => {
        onChange({ ...connection, keepalive_interval_seconds: parseNonNegativeInt(raw) });
    };

    return (
        <>
            <EditField label={t('home.devices.connTimeout')} value={resolveOptionalNumber(connection.connect_timeout_seconds)} onChange={handleConnectTimeoutChange} placeholder={t('home.devices.connTimeoutPlaceholder')} />
            <EditField label={t('home.devices.keepalive')} value={resolveDisplayNumber(connection.keepalive_interval_seconds)} onChange={handleKeepaliveIntervalChange} placeholder={t('home.devices.keepalivePlaceholder')} />
        </>
    );
}

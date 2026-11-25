import {useState} from 'react';
import {useI18n} from '@/common/i18n/useI18n.ts';
import {isNil} from "@/common/util/StringUtil.ts";
import {TextareaField} from '@/page/common/components/input/TextareaField.tsx';

export function EnvFields(props: { value: Record<string, string> | null | undefined; onChange: (value: Record<string, string> | null) => void }) {
    const {value, onChange} = props;
    const {t} = useI18n();
    const [text, setText] = useState<string>(() => {
        if (isNil(value)) {
            return '';
        }
        return Object.entries(value).map(([key, val]) => `${key}=${val}`).join('\n');
    });

    const parseText = (raw: string): Record<string, string> | null => {
        const lines = raw.split('\n').map(line => line.trim()).filter(line => line.length > 0);
        if (lines.length === 0) {
            return null;
        }
        const record: Record<string, string> = {};
        for (const line of lines) {
            const idx = line.indexOf('=');
            if (idx <= 0) {
                continue;
            }
            const key = line.slice(0, idx).trim();
            const val = line.slice(idx + 1).trim();
            if (key.length === 0) {
                continue;
            }
            record[key] = val;
        }
        if (Object.keys(record).length === 0) {
            return null;
        }
        return record;
    };

    const handleChange = (raw: string): void => {
        setText(raw);
        onChange(parseText(raw));
    };

    return <TextareaField label={t('home.devices.envVars')} value={text} onChange={handleChange} placeholder={t('home.devices.envPlaceholder')} minRows={3} maxRows={8}/>;
}

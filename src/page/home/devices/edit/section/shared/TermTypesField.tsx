import {useI18n} from '@/common/i18n/useI18n.ts';
import {isNil} from "@/common/util/StringUtil.ts";
import {TERMINAL_OPTIONS} from "@/common/const/DeviceConst.ts";
import {MultiSelectField} from '@/page/common/components/input/MultiSelectField.tsx';

export function TermTypesField(props: { value: string[] | null | undefined; onChange: (value: string[] | null) => void }) {
    const {value, onChange} = props;
    const {t} = useI18n();

    const resolveOptions = () => {
        return TERMINAL_OPTIONS.filter(opt => opt.value.length > 0);
    };

    const resolveValue = (): string[] => {
        if (isNil(value)) {
            return [];
        }
        return value;
    };

    const handleChange = (next: string[]): void => {
        if (next.length === 0) {
            onChange(null);
            return;
        }
        onChange(next);
    };

    return <MultiSelectField label={t('home.devices.termTypes')} value={resolveValue()} onChange={handleChange} options={resolveOptions()} placeholder={t('home.devices.termTypesPlaceholder')}/>;
}

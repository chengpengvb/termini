import {useI18n} from '@/common/i18n/useI18n.ts';
import {TERMINAL_OPTIONS} from "@/common/const/DeviceConst.ts";
import { SelectField } from '@/page/common/components/input/SelectField.tsx';

export function TermFields(props: { value?: string | null; onChange: (value: string) => void; placeholder?: string }) {
    const { value, onChange, placeholder } = props;
    const {t} = useI18n();
    return <SelectField label={t('home.common.terminal')} value={value} onChange={onChange} options={TERMINAL_OPTIONS} placeholder={placeholder} />;
}

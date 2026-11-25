import {Section} from './shared/Section.tsx';
import {useI18n} from '@/common/i18n/useI18n.ts';
import type {ConfigKeyPair} from '@/common/model/ConfigKeyPair.ts';
import {EditField} from '@/page/common/components/input/EditField.tsx';

export function GeneralSection(props: { value: ConfigKeyPair; onChange: (value: ConfigKeyPair) => void }) {
    const {value, onChange} = props;
    const {t} = useI18n();

    const handleNameChange = (name: string): void => {
        onChange({...value, name});
    };

    return (
        <Section title={t('home.keys.generalSection')}>
            <EditField label={t('home.common.name')} value={value.name} onChange={handleNameChange} placeholder={t('home.keys.namePlaceholder')}/>
        </Section>
    );
}

import {Section} from './shared/Section.tsx';
import {useI18n} from '@/common/i18n/useI18n.ts';
import {TextAreaField} from './shared/TextAreaField.tsx';
import type {ConfigKeyPair} from '@/common/model/ConfigKeyPair.ts';
import {EditField} from '@/page/common/components/input/EditField.tsx';
import {FoldSection} from '@/page/common/components/section/FoldSection.tsx';

export function KeyMaterialSection(props: { value: ConfigKeyPair; onChange: (value: ConfigKeyPair) => void }) {
    const {t} = useI18n();
    const {value, onChange} = props;

    const handlePublicKeyChange = (public_key: string): void => {
        onChange({...value, public_key});
    };

    const handlePrivateKeyChange = (private_key: string): void => {
        onChange({...value, private_key});
    };

    const handlePassphraseChange = (passphrase: string): void => {
        onChange({...value, passphrase});
    };

    const handleCertificateChange = (certificate: string): void => {
        onChange({...value, certificate});
    };

    return (
        <Section title={t('home.keys.materialSection')}>
            <TextAreaField label={t('home.keys.publicKeyContent')} value={value.public_key} onChange={handlePublicKeyChange} placeholder={t('home.keys.publicKeyPlaceholder')} rows={3} copyable importable accept=".pub,.pem,.openssh,text/plain"/>
            <TextAreaField label={t('home.keys.privateKeyContent')} value={value.private_key} onChange={handlePrivateKeyChange} placeholder={t('home.keys.privateKeyPlaceholder')} rows={6} password copyable importable accept=".pem,.key,.openssh,.id_rsa,.id_ed25519,.id_ecdsa,.ppk,text/plain"/>
            <EditField label={t('home.keys.passphrase')} value={value.passphrase} onChange={handlePassphraseChange} placeholder={t('home.keys.passphrasePlaceholder')} password/>
            <FoldSection label={t('home.keys.moreParams')}>
                <TextAreaField label={t('home.keys.certContent')} value={value.certificate} onChange={handleCertificateChange} placeholder={t('home.keys.certPlaceholder')} rows={2} copyable importable accept=".pub,.crt,.cer,.pem,text/plain"/>
            </FoldSection>
        </Section>
    );
}

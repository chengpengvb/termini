import {useRef, useState} from 'react';
import {useI18n} from '@/common/i18n/useI18n.ts';
import type {ChangeEvent, DragEvent} from 'react';
import {Box, ButtonBase, Typography} from '@mui/material';
import {isNotEmpty, isNotNil} from '@/common/util/StringUtil.ts';
import {AlertDialog} from '@/common/components/dialog/AlertDialog.tsx';
import {ContentCopy, Visibility, VisibilityOff} from '@mui/icons-material';

const FILE_SIZE_LIMIT_BYTES = 200 * 1024;

export function TextAreaField(props: { label?: string; value: string | undefined; onChange: (value: string) => void; placeholder?: string; rows?: number; password?: boolean; copyable?: boolean; importable?: boolean; accept?: string }) {
    const {t} = useI18n();
    const fileInputRef = useRef<HTMLInputElement>(null);
    const [revealed, setRevealed] = useState<boolean>(false);
    const [dragActive, setDragActive] = useState<boolean>(false);
    const [fileInputVersion, setFileInputVersion] = useState<number>(0);
    const {label, value, onChange, placeholder, rows, password, copyable, importable, accept} = props;
    const [alertInfo, setAlertInfo] = useState<{ open: boolean; type: 'success' | 'error'; message: string }>({open: false, type: 'success', message: ''});

    const resolveRows = (): number => {
        if (isNotNil(rows)) {
            return rows;
        }
        return 4;
    };

    const canCopy = (): boolean => {
        return copyable === true;
    };

    const canImport = (): boolean => {
        return importable === true;
    };

    const hasActions = (): boolean => {
        return password === true || canCopy() || canImport();
    };

    const isMasked = (): boolean => {
        return password === true && !revealed;
    };

    const resolveRootClass = (): string => {
        if (dragActive) {
            return 'tm-edit-textarea is-drag';
        }
        return 'tm-edit-textarea';
    };

    const resolveInputClass = (): string => {
        if (isMasked()) {
            return 'tm-edit-textarea-input is-masked';
        }
        return 'tm-edit-textarea-input';
    };

    const resolveInputValue = (): string => {
        if (isNotNil(value)) {
            return value;
        }
        return '';
    };

    const showAlert = (type: 'success' | 'error', message: string): void => {
        setAlertInfo({open: true, type, message});
    };

    const handleAlertClose = (): void => {
        setAlertInfo({open: false, type: alertInfo.type, message: alertInfo.message});
    };

    const handleToggleReveal = (): void => {
        setRevealed(!revealed);
    };

    const handleCopy = async (): Promise<void> => {
        if (!isNotEmpty(value)) {
            return;
        }
        await navigator.clipboard.writeText(value);
        showAlert('success', t('home.keys.copySuccess'));
    };

    const handleImportClick = (): void => {
        const el = fileInputRef.current;
        if (!el) {
            return;
        }
        el.click();
    };

    const handleFileLoad = async (file: File): Promise<void> => {
        if (file.size > FILE_SIZE_LIMIT_BYTES) {
            showAlert('error', t('home.keys.importTooLarge'));
            return;
        }
        if (file.name.toLowerCase().endsWith('.ppk')) {
            showAlert('error', t('home.keys.importPpkError'));
            return;
        }
        const text = await file.text();
        onChange(text);
        showAlert('success', t('home.keys.importSuccess'));
    };

    const handleFileChange = async (event: ChangeEvent<HTMLInputElement>): Promise<void> => {
        const files = event.target.files;
        if (!files) {
            return;
        }
        const file = files[0];
        if (!file) {
            return;
        }
        setFileInputVersion(fileInputVersion + 1);
        await handleFileLoad(file);
    };

    const handleDragOver = (event: DragEvent<HTMLDivElement>): void => {
        if (importable !== true) {
            return;
        }
        event.preventDefault();
        event.stopPropagation();
        setDragActive(true);
    };

    const handleDragLeave = (event: DragEvent<HTMLDivElement>): void => {
        if (importable !== true) {
            return;
        }
        event.preventDefault();
        event.stopPropagation();
        setDragActive(false);
    };

    const handleDrop = async (event: DragEvent<HTMLDivElement>): Promise<void> => {
        if (importable !== true) {
            return;
        }
        event.preventDefault();
        event.stopPropagation();
        setDragActive(false);
        const files = event.dataTransfer.files;
        if (!files || files.length === 0) {
            return;
        }
        const file = files[0];
        await handleFileLoad(file);
    };

    return (
        <Box className={resolveRootClass()} onDragOver={handleDragOver} onDragLeave={handleDragLeave} onDrop={event => { void handleDrop(event); }}>
            {isNotNil(label) || hasActions() ? (
                <Box className="tm-edit-textarea-head">
                    {isNotNil(label) ? (
                        <Typography className="tm-edit-textarea-label">{label}</Typography>
                    ) : <Box className="tm-edit-textarea-label-spacer"/>}
                    {hasActions() ? (
                        <Box className="tm-edit-textarea-actions">
                            {canImport() ? (
                                <ButtonBase className="tm-edit-textarea-import" onClick={handleImportClick} aria-label={t('home.keys.importFromFile')}>
                                    {t('home.keys.importFromFile')}
                                </ButtonBase>
                            ) : null}
                            {password === true ? (
                                <ButtonBase className="tm-edit-field-action" onClick={handleToggleReveal} aria-label={revealed ? t('home.keys.hide') : t('home.keys.show')}>
                                    {revealed ? <VisibilityOff/> : <Visibility/>}
                                </ButtonBase>
                            ) : null}
                            {canCopy() ? (
                                <ButtonBase className="tm-edit-field-action" onClick={() => { void handleCopy(); }} aria-label={t('home.keys.copy')}>
                                    <ContentCopy/>
                                </ButtonBase>
                            ) : null}
                            {importable === true ? (
                                <Box component="input" key={fileInputVersion} ref={fileInputRef} type="file" accept={accept} onChange={event => { void handleFileChange(event); }} className="tm-edit-textarea-file"/>
                            ) : null}
                        </Box>
                    ) : null}
                </Box>
            ) : null}
            <Box className={resolveInputClass()} component="textarea" rows={resolveRows()} value={resolveInputValue()} onChange={(event: ChangeEvent<HTMLTextAreaElement>) => onChange(event.target.value)} placeholder={placeholder}/>
            <AlertDialog open={alertInfo.open} duration={1500} type={alertInfo.type} message={alertInfo.message} handleClose={handleAlertClose}/>
        </Box>
    );
}

import {useI18n} from '@/common/i18n/useI18n.ts';
import {ArrowDropDown} from '@mui/icons-material';
import {isEmpty} from '@/common/util/StringUtil.ts';
import {useEffect, useMemo, useRef, useState} from 'react';
import {usePlatformMobile} from '@/common/context/usePlatformMobile';
import {EditField} from '@/page/common/components/input/EditField.tsx';
import {useJobIndicatorService} from '@/common/context/useAppHooks.tsx';
import {SelectField} from '@/page/common/components/input/SelectField.tsx';
import {ActionSheet} from '@/page/common/components/dialog/ActionSheet.tsx';
import {TextareaField} from '@/page/common/components/input/TextareaField.tsx';
import {Box, InputBase, Menu, MenuItem, MenuList, Typography} from '@mui/material';
import {DEFAULT_REGEX_EXPR, DEFAULT_SHELL_SCRIPT, DEFAULT_SNMP_SPEC, indicatorCommand, indicatorUnit} from '@/page/common/util/jobUtil.ts';
import {buildIndicatorArg, ICMP_COUNT_DEFAULT, INDICATOR_ARG_TYPE, type IndicatorArgTemplate, JOB_PARSER_TYPE, JOB_PROTOCOL, type JobIndicator, protocolLabel, TLS_CERT_PORT_DEFAULT} from '@/common/model/JobIndicator.ts';

export interface ItemExecuteProps {
    draft: JobIndicator;
    intervalSecs: number;
    onIntervalChange: (secs: number) => void;
    onPatch: (next: Partial<JobIndicator>) => void;
}

const INTERVAL_MIN_MINUTES = 1;

const INTERVAL_OPTIONS = [
    {value: '1', label: '1m'},
    {value: '2', label: '2m'},
    {value: '5', label: '5m'},
    {value: '15', label: '15m'},
    {value: '60', label: '1h'},
];

function toIntervalMinutes(secs: number): number {
    const minutes = Math.trunc(secs / 60);
    if (minutes < INTERVAL_MIN_MINUTES) {
        return INTERVAL_MIN_MINUTES;
    }
    return minutes;
}

function IntervalField(props: { valueSecs: number; onChangeSecs: (secs: number) => void }) {
    const {t} = useI18n();
    const mobileViewport = usePlatformMobile();
    const fieldRef = useRef<HTMLDivElement | null>(null);
    const [anchorEl, setAnchorEl] = useState<HTMLElement | null>(null);
    const [text, setText] = useState<string>(() => String(toIntervalMinutes(props.valueSecs)));
    const open = Boolean(anchorEl);
    const {valueSecs, onChangeSecs} = props;

    function commit(raw: string): void {
        const parsed = Number(raw.trim());
        if (!Number.isFinite(parsed)) {
            setText(String(toIntervalMinutes(valueSecs)));
            return;
        }
        const minutes = Math.max(INTERVAL_MIN_MINUTES, Math.trunc(parsed));
        setText(String(minutes));
        onChangeSecs(minutes * 60);
    }

    function handleOpen(): void {
        setAnchorEl(fieldRef.current);
    }

    function handleClose(): void {
        setAnchorEl(null);
    }

    function handlePick(minutes: string): void {
        commit(minutes);
        handleClose();
    }

    function renderOptionList(mobile: boolean) {
        return mobile ? (
            <MenuList disablePadding className="tm-action-sheet-list">
                {INTERVAL_OPTIONS.map(opt => (
                    <MenuItem key={opt.value} className={String(toIntervalMinutes(valueSecs)) === opt.value ? 'tm-action-sheet-item is-selected' : 'tm-action-sheet-item'} onClick={() => handlePick(opt.value)}>
                        <Box component="span" className="tm-menu-item-text">{opt.label}</Box>
                    </MenuItem>
                ))}
            </MenuList>
        ) : (
            <Box sx={{maxHeight: 280, overflowY: 'auto'}}>
                {INTERVAL_OPTIONS.map(opt => (
                    <MenuItem key={opt.value} className={String(toIntervalMinutes(valueSecs)) === opt.value ? 'tm-menu-item is-selected' : 'tm-menu-item'} onClick={() => handlePick(opt.value)} sx={{minWidth: 0, maxWidth: '100%'}}>
                        <Box sx={{minWidth: 0, width: '100%', overflow: 'clip', textOverflow: 'ellipsis', whiteSpace: 'nowrap'}}>{opt.label}</Box>
                    </MenuItem>
                ))}
            </Box>
        );
    }

    useEffect(() => {
        setText(String(toIntervalMinutes(valueSecs)));
    }, [valueSecs]);

    return (
        <Box>
            <Box ref={fieldRef} className="tm-edit-field" sx={{display: 'flex', alignItems: 'center', width: '100%', maxWidth: '100%', minWidth: 0, flexShrink: 0, boxSizing: 'border-box', minHeight: '38px', border: '1px solid var(--tm-border-subtle)', borderRadius: '7px', bgcolor: 'transparent', transition: 'border-color 120ms ease, background-color 120ms ease', '&:hover': {bgcolor: 'var(--tm-bg-hover)'}, '&:focus-within': {bgcolor: 'var(--tm-bg-hover)', borderColor: 'var(--tm-border-accent)'}}}>
                <Typography className="tm-edit-field-label" sx={{fontSize: 13, color: 'var(--tm-text-tertiary)', pl: 1, width: 72, flexShrink: 0, whiteSpace: 'nowrap', userSelect: 'none'}}>{t('home.job.intervalLabel')}</Typography>
                <InputBase className="tm-edit-field-input" value={text} onChange={e => setText(e.target.value)} onBlur={() => commit(text)} placeholder={t('home.job.intervalPlaceholder')}
                           sx={{flex: 1, minWidth: 0, px: 1, py: 0.5, fontSize: 13, color: 'var(--tm-text-primary)', '& input': {minWidth: 0}, '& input::placeholder': {color: 'var(--tm-text-tertiary)', opacity: 1}}}/>
                <ArrowDropDown className={open ? 'tm-edit-field-caret is-open' : 'tm-edit-field-caret'} onClick={handleOpen}/>
            </Box>
            {mobileViewport ? (
                <ActionSheet open={open} onClose={handleClose}>
                    {renderOptionList(true)}
                </ActionSheet>
            ) : (
                <Menu open={open} anchorEl={anchorEl} onClose={handleClose} anchorOrigin={{vertical: 'bottom', horizontal: 'center'}} transformOrigin={{vertical: 'top', horizontal: 'center'}}
                      slotProps={{paper: {className: 'tm-menu-paper', sx: {width: anchorEl ? Math.max(anchorEl.clientWidth, 220) : 220, mt: 0.5, maxWidth: 360}}, list: {className: 'tm-menu-list', sx: {py: 0.5}}}}>
                    {renderOptionList(false)}
                </Menu>
            )}
        </Box>
    );
}

export function ItemExecute(props: ItemExecuteProps) {
    const {draft, intervalSecs, onPatch, onIntervalChange} = props;
    const unit = indicatorUnit(draft);
    const jobIndicatorService = useJobIndicatorService();
    const {t} = useI18n();
    const [templates, setTemplates] = useState<IndicatorArgTemplate[]>([]);
    const [defaultIndicators, setDefaultIndicators] = useState<JobIndicator[]>([]);
    const argTemplate = templates.find(item => item.protocol === draft.protocol) ?? draft.arg?.template ?? null;
    const protocolOptions = templates.map(item => ({value: String(item.protocol), label: protocolLabel(item.protocol)}));
    const snmpTemplates = defaultIndicators.filter(item => item.protocol === JOB_PROTOCOL.SNMP);
    const enabledOptions = useMemo(() => [
        {value: '1', label: t('home.job.collectOn')},
        {value: '0', label: t('home.job.collectOff')},
    ], [t]);

    const patchArg = (value: string, nextUnit: string): void => {
        if (argTemplate == null) {
            return;
        }
        onPatch({arg: buildIndicatorArg(argTemplate, value, nextUnit)});
    };

    const handleProtocolChange = (raw: string): void => {
        const protocol = Number(raw);
        const nextTemplate = templates.find(item => item.protocol === protocol);
        if (nextTemplate == null || protocol === draft.protocol) {
            return;
        }
        if (protocol === JOB_PROTOCOL.HTTP) {
            onPatch({protocol, arg: buildIndicatorArg(nextTemplate, '', 'ms'), parser_type: JOB_PARSER_TYPE.EXIT_CODE, parse_expr: ''});
            return;
        }
        if (protocol === JOB_PROTOCOL.TCP) {
            onPatch({protocol, arg: buildIndicatorArg(nextTemplate, '', 'ms'), parser_type: JOB_PARSER_TYPE.EXIT_CODE, parse_expr: ''});
            return;
        }
        if (protocol === JOB_PROTOCOL.TLS_CERT) {
            onPatch({protocol, arg: buildIndicatorArg(nextTemplate, TLS_CERT_PORT_DEFAULT, 'd'), parser_type: JOB_PARSER_TYPE.EXIT_CODE, parse_expr: ''});
            return;
        }
        if (nextTemplate.argType === INDICATOR_ARG_TYPE.NUMBER) {
            onPatch({protocol, arg: buildIndicatorArg(nextTemplate, ICMP_COUNT_DEFAULT, 'ms'), parser_type: JOB_PARSER_TYPE.EXIT_CODE, parse_expr: ''});
            return;
        }
        if (nextTemplate.argType === INDICATOR_ARG_TYPE.SNMP) {
            onPatch({protocol, arg: buildIndicatorArg(nextTemplate, DEFAULT_SNMP_SPEC, ''), parser_type: JOB_PARSER_TYPE.EXIT_CODE, parse_expr: ''});
            return;
        }
        const command = indicatorCommand(draft);
        const wasShell = argTemplate?.argType === INDICATOR_ARG_TYPE.SHELL;
        const nextCommand = isEmpty(command) || !wasShell ? DEFAULT_SHELL_SCRIPT : command;
        const parserType = draft.parser_type === JOB_PARSER_TYPE.EXIT_CODE ? JOB_PARSER_TYPE.REGEX : draft.parser_type;
        const nextUnit = wasShell ? unit : '';
        onPatch({
            protocol,
            parser_type: parserType,
            arg: buildIndicatorArg(nextTemplate, nextCommand, nextUnit),
            parse_expr: isEmpty(draft.parse_expr) ? DEFAULT_REGEX_EXPR : draft.parse_expr,
        });
    };

    const handleSnmpTemplatePick = (indicatorId: string): void => {
        const template = snmpTemplates.find(item => item.id === indicatorId);
        if (template == null || argTemplate == null) {
            return;
        }
        onPatch({
            name: template.name,
            parse_expr: '',
            parser_type: JOB_PARSER_TYPE.EXIT_CODE,
            display_unit: template.display_unit,
            transform_expr: template.transform_expr,
            arg: buildIndicatorArg(argTemplate, template.arg?.value ?? DEFAULT_SNMP_SPEC, template.arg?.unit ?? ''),
        });
    };

    const renderSnmpTemplateField = () => {
        if (argTemplate?.argType !== INDICATOR_ARG_TYPE.SNMP || snmpTemplates.length === 0) {
            return null;
        }
        const options = snmpTemplates.map(item => ({value: item.id, label: item.name}));
        const selected = snmpTemplates.find(item => item.arg?.value === draft.arg?.value);
        return <SelectField label={t('home.job.snmpTemplate')} labelWidth={72} value={selected?.id ?? ''} onChange={handleSnmpTemplatePick} options={options} placeholder={t('home.job.snmpTemplateNone')}/>;
    };

    const renderArgField = () => {
        if (argTemplate == null) {
            return null;
        }
        const label = t(argTemplate.labelI18n);
        if (draft.protocol === JOB_PROTOCOL.HTTP) {
            return <EditField label={label} labelWidth={72} value={indicatorCommand(draft)} onChange={v => patchArg(v, unit)} placeholder={t('home.job.httpUrlPlaceholder')}/>;
        }
        if (draft.protocol === JOB_PROTOCOL.TCP) {
            return <EditField label={label} labelWidth={72} value={indicatorCommand(draft)} onChange={v => patchArg(v, unit)} placeholder={t('home.job.tcpPortPlaceholder')}/>;
        }
        if (draft.protocol === JOB_PROTOCOL.TLS_CERT) {
            return <EditField label={label} labelWidth={72} value={indicatorCommand(draft)} onChange={v => patchArg(v, unit)} placeholder={t('home.job.tlsPortPlaceholder')}/>;
        }
        if (argTemplate.argType === INDICATOR_ARG_TYPE.NUMBER) {
            return <EditField label={label} labelWidth={72} value={indicatorCommand(draft)} onChange={v => patchArg(v, unit)} placeholder={t('home.job.icmpPlaceholder')}/>;
        }
        if (argTemplate.argType === INDICATOR_ARG_TYPE.SNMP) {
            return <TextareaField label={label} labelWidth={72} value={indicatorCommand(draft)} onChange={v => patchArg(v, unit)} placeholder={t('home.job.snmpPlaceholder')} expandable language="json" minRows={1} maxRows={1}/>;
        }
        return <TextareaField label={label} labelWidth={72} value={indicatorCommand(draft)} onChange={v => patchArg(v, unit)} placeholder={t('home.job.shellPlaceholder')} expandable language="bash" minRows={1} maxRows={1}/>;
    };

    useEffect(() => {
        let cancelled = false;
        void jobIndicatorService.selectArgTemplate().then(list => {
            if (cancelled) {
                return;
            }
            setTemplates(list);
        }).catch(() => {
            if (cancelled) {
                return;
            }
            setTemplates([]);
        });
        void jobIndicatorService.selectDefault().then(list => {
            if (cancelled) {
                return;
            }
            setDefaultIndicators(list);
        }).catch(() => {
            if (cancelled) {
                return;
            }
            setDefaultIndicators([]);
        });
        return () => {
            cancelled = true;
        };
    }, [jobIndicatorService]);

    return (
        <Box className="tm-job-detail-section">
            <Typography className="tm-job-section-title">{t('home.job.executeSection')}</Typography>
            <EditField label={t('home.common.name')} labelWidth={72} value={draft.name} onChange={v => onPatch({name: v})} placeholder={t('home.job.metricNamePlaceholder')}/>
            <SelectField label={t('home.job.protocol')} labelWidth={72} value={String(draft.protocol)} onChange={handleProtocolChange} options={protocolOptions}/>
            {renderSnmpTemplateField()}
            {renderArgField()}
            <EditField label={t('home.job.unit')} labelWidth={72} value={unit} onChange={v => patchArg(indicatorCommand(draft), v)} placeholder={t('home.job.unitPlaceholder')}/>
            <IntervalField valueSecs={intervalSecs} onChangeSecs={onIntervalChange}/>
            <SelectField label={t('home.job.collect')} labelWidth={72} value={draft.enabled ? '1' : '0'} onChange={v => onPatch({enabled: v === '1'})} options={enabledOptions}/>
        </Box>
    );
}

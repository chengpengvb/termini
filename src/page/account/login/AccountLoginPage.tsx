import logoUrl from '@/assets/icon/logo.svg';
import {translate} from '@/common/i18n/i18n.ts';
import type {FormEvent, ReactNode} from 'react';
import {useI18n} from '@/common/i18n/useI18n.ts';
import {useEffect, useRef, useState} from 'react';
import {isEmpty} from '@/common/util/StringUtil.ts';
import {APP_THEME} from '@/common/model/AppTheme.ts';
import {AlertDialog} from '@/common/components/dialog/AlertDialog.tsx';
import {AppThemeProvider} from '@/common/context/AppThemeProvider.tsx';
import {EditField} from '@/page/common/components/input/EditField.tsx';
import {Box, ButtonBase, CircularProgress, Typography} from '@mui/material';
import {BizError, resolveErrorText} from '@/common/model/resp/ResultModel.ts';
import {usePlatformNativeMobile} from '@/common/context/usePlatformMobile.ts';
import {BarControls} from '@/common/components/tab/components/tabBar/BarControls.tsx';
import {useAccountService, useSystemUtilService, useTerminalThemeService} from '@/common/context/useAppHooks.tsx';

const ALERT_KIND = {SUCCESS: 0, ERROR: 1} as const;
const LOGIN_VIEW = {LOGIN: 0, REGISTER: 1, RESET: 2} as const;

function resolveLoginError(err: unknown): string {
    if (err instanceof TypeError) {
        return translate('account.login.networkError');
    }
    if (err instanceof BizError) {
        return resolveErrorText(err);
    }
    return translate('account.login.systemError');
}

interface AccountLoginPageProps {
    onDismiss: () => void;
}

export default function AccountLoginPage(props: AccountLoginPageProps) {
    const {t} = useI18n();
    const {onDismiss} = props;
    const [code, setCode] = useState('');
    const [email, setEmail] = useState('');
    const accountService = useAccountService();
    const [codeLeft, setCodeLeft] = useState(0);
    const [password, setPassword] = useState('');
    const nativeMobile = usePlatformNativeMobile();
    const systemUtilService = useSystemUtilService();
    const rootRef = useRef<HTMLElement | null>(null);
    const [alertOpen, setAlertOpen] = useState(false);
    const [webLoading, setWebLoading] = useState(false);
    const [sendLoading, setSendLoading] = useState(false);
    const terminalThemeService = useTerminalThemeService();
    const [webLoginReady, setWebLoginReady] = useState(false);
    const [submitLoading, setSubmitLoading] = useState(false);
    const [view, setView] = useState<number>(LOGIN_VIEW.LOGIN);
    const [alertMessage, setAlertMessage] = useState<string | null>(null);
    const [alertType, setAlertType] = useState<number>(ALERT_KIND.ERROR);
    const [appThemeId, setAppThemeId] = useState(() => terminalThemeService.getAppThemeId());

    function showAlert(type: number, message: string): void {
        setAlertType(type);
        setAlertMessage(message);
        setAlertOpen(true);
    }

    function handleAlertClose(): void {
        setAlertOpen(false);
        setAlertMessage(null);
    }

    function handleWebLogin(): void {
        setWebLoading(true);
        accountService
            .login()
            .then(() => {
                setWebLoading(true);
            })
            .catch((e: unknown) => {
                showAlert(ALERT_KIND.ERROR, resolveLoginError(e));
                setWebLoading(false);
            });
    }

    function handlePasswordLogin(): void {
        if (isEmpty(email) || isEmpty(password)) {
            showAlert(ALERT_KIND.ERROR, t('account.login.emailPasswordRequired'));
            return;
        }
        setSubmitLoading(true);
        accountService
            .loginWithPassword(email.trim(), password)
            .then(() => {
                showAlert(ALERT_KIND.SUCCESS, t('account.login.loginSuccess'));
            })
            .catch((e: unknown) => {
                showAlert(ALERT_KIND.ERROR, resolveLoginError(e));
            })
            .finally(() => {
                setSubmitLoading(false);
            });
    }

    function handleRegister(): void {
        if (isEmpty(email) || isEmpty(password) || isEmpty(code)) {
            showAlert(ALERT_KIND.ERROR, t('account.login.registerRequired'));
            return;
        }
        setSubmitLoading(true);
        accountService
            .register(email.trim(), password, code.trim())
            .then(() => {
                showAlert(ALERT_KIND.SUCCESS, t('account.login.registerSuccess'));
            })
            .catch((e: unknown) => {
                showAlert(ALERT_KIND.ERROR, resolveLoginError(e));
            })
            .finally(() => {
                setSubmitLoading(false);
            });
    }

    function handleResetPassword(): void {
        if (isEmpty(email) || isEmpty(password) || isEmpty(code)) {
            showAlert(ALERT_KIND.ERROR, t('account.login.registerRequired'));
            return;
        }
        setSubmitLoading(true);
        accountService
            .resetPassword(email.trim(), password, code.trim())
            .then(() => {
                setCode('');
                setView(LOGIN_VIEW.LOGIN);
                showAlert(ALERT_KIND.SUCCESS, t('account.login.resetSuccess'));
            })
            .catch((e: unknown) => {
                showAlert(ALERT_KIND.ERROR, resolveLoginError(e));
            })
            .finally(() => {
                setSubmitLoading(false);
            });
    }

    function resolveSendAction(): ReactNode {
        if (sendLoading) {
            return <CircularProgress size={14}/>;
        }
        if (codeLeft > 0) {
            return `${codeLeft}s`;
        }
        return t('account.login.sendCode');
    }

    function handleSendCode(): void {
        if (codeLeft > 0 || sendLoading) {
            return;
        }
        if (isEmpty(email)) {
            showAlert(ALERT_KIND.ERROR, t('account.login.sendCodeEmailRequired'));
            return;
        }
        setSendLoading(true);
        const sending = view === LOGIN_VIEW.RESET ? accountService.sendResetCode(email.trim()) : accountService.sendRegisterCode(email.trim());
        sending
            .then(() => {
                setCodeLeft(60);
            })
            .catch((e: unknown) => {
                showAlert(ALERT_KIND.ERROR, resolveLoginError(e));
            })
            .finally(() => {
                setSendLoading(false);
            });
    }

    function handleSubmit(event: FormEvent<HTMLFormElement>): void {
        event.preventDefault();
        if (view === LOGIN_VIEW.LOGIN) {
            handlePasswordLogin();
            return;
        }
        if (view === LOGIN_VIEW.RESET) {
            handleResetPassword();
            return;
        }
        handleRegister();
    }

    function resolveSubmitLabel(): string {
        if (view === LOGIN_VIEW.LOGIN) {
            return t('account.login.signIn');
        }
        if (view === LOGIN_VIEW.RESET) {
            return t('account.login.resetPassword');
        }
        return t('account.login.register');
    }

    function resolveSwitchPrompt(): string {
        if (view === LOGIN_VIEW.LOGIN) {
            return t('account.login.noAccount');
        }
        return t('account.login.hasAccount');
    }

    function resolveSwitchAction(): string {
        if (view === LOGIN_VIEW.LOGIN) {
            return t('account.login.register');
        }
        return t('account.login.signIn');
    }

    function handleOpenReset(): void {
        setView(LOGIN_VIEW.RESET);
    }

    function handleSwitchAccount(): void {
        setView(view === LOGIN_VIEW.LOGIN ? LOGIN_VIEW.REGISTER : LOGIN_VIEW.LOGIN);
    }

    function handleClose(): void {
        onDismiss();
    }

    useEffect(() => {
        const syncTheme = (): void => setAppThemeId(terminalThemeService.getAppThemeId());
        const off = terminalThemeService.addChangeListener(syncTheme);
        syncTheme();
        return off;
    }, [terminalThemeService]);

    useEffect(() => {
        void systemUtilService.getPlatformInfo().then(info => {
            setWebLoginReady(info.has_web_login);
        });
    }, [systemUtilService]);

    useEffect(() => {
        return accountService.addLoginErrorListener(message => {
            setWebLoading(false);
            showAlert(ALERT_KIND.ERROR, message);
        });
    }, [accountService]);

    useEffect(() => {
        if (codeLeft <= 0) {
            return;
        }
        const timer = window.setTimeout(() => {
            setCodeLeft(codeLeft - 1);
        }, 1000);
        return () => {
            window.clearTimeout(timer);
        };
    }, [codeLeft]);

    useEffect(() => {
        const onKeyDown = (event: KeyboardEvent): void => {
            if (event.key !== 'Escape' || event.defaultPrevented || rootRef.current?.querySelector('[role="dialog"]')) {
                return;
            }
            event.preventDefault();
            onDismiss();
        };
        document.addEventListener('keydown', onKeyDown, true);
        return () => {
            document.removeEventListener('keydown', onKeyDown, true);
        };
    }, [onDismiss]);

    return (
        <AppThemeProvider appearance={appThemeId === APP_THEME.LIGHT ? APP_THEME.LIGHT : APP_THEME.DEFAULT}>
            <Box ref={rootRef} className="tm-account-login is-overlay" data-account-theme={appThemeId} role="dialog" aria-modal="true" aria-label={resolveSubmitLabel()}>
                {!nativeMobile ? (
                    <Box className="tm-account-login-titlebar">
                        <Box className="tm-account-login-drag" onMouseDown={() => void systemUtilService.startWindowDragging()}/>
                        <BarControls dialogContainerRef={rootRef}/>
                    </Box>
                ) : null}
                <Box className="tm-account-login-stage">
                    <Box className="tm-account-login-panel">
                        <Box className="tm-account-login-brand">
                            <Box component="img" className="tm-account-login-logo" src={logoUrl} alt="Termini"/>
                            <Typography className="tm-account-login-title" component="h1">
                                {t('account.login.title')}
                            </Typography>
                        </Box>
                        <Box component="form" className="tm-account-login-form" autoComplete="off" onSubmit={handleSubmit}>
                            <EditField label={t('account.login.email')} value={email} onChange={setEmail}/>
                            {view === LOGIN_VIEW.LOGIN ? null : (
                                <Box className={codeLeft > 0 || sendLoading ? 'tm-account-login-code is-wait' : 'tm-account-login-code'}>
                                    <EditField label={t('account.login.code')} value={code} onChange={setCode} actionLabel={t('account.login.sendCode')} actionIcon={resolveSendAction()} onAction={handleSendCode}/>
                                </Box>
                            )}
                            <EditField label={view === LOGIN_VIEW.RESET ? t('account.login.newPassword') : t('account.login.password')} value={password} onChange={setPassword} password/>
                            {view === LOGIN_VIEW.LOGIN ? (
                                <ButtonBase className="tm-account-login-forgot" type="button" onClick={handleOpenReset}>
                                    {t('account.login.forgotPassword')}
                                </ButtonBase>
                            ) : null}
                            <ButtonBase className="tm-account-login-btn" type="submit" disabled={submitLoading}>
                                {submitLoading ? <CircularProgress size={16}/> : null}
                                {resolveSubmitLabel()}
                            </ButtonBase>
                            {view === LOGIN_VIEW.LOGIN && webLoginReady ? (
                                <ButtonBase className="tm-account-login-web" type="button" onClick={handleWebLogin}>
                                    {webLoading ? t('account.login.waiting') : t('account.login.webLogin')}
                                </ButtonBase>
                            ) : null}
                        </Box>
                    </Box>
                </Box>
                <Box className="tm-account-login-switch">
                    <Box component="span">
                        {resolveSwitchPrompt()}
                        <ButtonBase className="tm-account-login-switch-btn" type="button" onClick={handleSwitchAccount}>
                            {resolveSwitchAction()}
                        </ButtonBase>
                    </Box>
                    <ButtonBase type="button" onClick={handleClose}>
                        {t('common.cancel')}
                    </ButtonBase>
                </Box>
                <AlertDialog duration={alertType === ALERT_KIND.SUCCESS ? 1500 : 8000} open={alertOpen} handleClose={handleAlertClose} message={alertMessage} type={alertType === ALERT_KIND.SUCCESS ? 'success' : 'error'} containerRef={rootRef}/>
            </Box>
        </AppThemeProvider>
    );
}

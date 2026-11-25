import { useEffect, useState } from 'react';
import { TabRoot } from '@/common/components/tab';
import { GuidanceRoot } from '@/page/guidance/GuidanceRoot.tsx';
import { AlertDialog } from '@/common/components/dialog/AlertDialog.tsx';
import AccountLoginPage from '@/page/account/login/AccountLoginPage.tsx';
import { DeckInputGuidance } from '@/page/guidance/DeckInputGuidance.tsx';
import type { UserAccount } from '@/common/model/resp/RespUserAccount.ts';
import { useAccountService, useTabManagerService, useUserService } from '@/common/context/useAppHooks.tsx';

export default function App() {
  const userService = useUserService();
  const manager = useTabManagerService();
  const accountService = useAccountService();
  const [loginError, setLoginError] = useState<string | null>(null);
  const [loginOpen, setLoginOpen] = useState<boolean>(() => userService.isLoginOpen());
  const [account, setAccount] = useState<UserAccount | null | undefined>(() => accountService.getAccount());
  const signedIn = account != null;
  const loginVisible = account === null && loginOpen;

  useEffect(() => {
    const unsub = accountService.addAccountChangeListener(setAccount);
    setAccount(accountService.getAccount());
    return unsub;
  }, [accountService]);

  useEffect(() => {
    return accountService.addLoginErrorListener(setLoginError);
  }, [accountService]);

  useEffect(() => {
    return userService.addLoginListener(setLoginOpen);
  }, [userService]);

  useEffect(() => {
    if (!signedIn) {
      return;
    }
    userService.setLoginOpen(false);
  }, [signedIn, userService]);

  const loginErrorDialog = <AlertDialog duration={8000} open={loginError != null} handleClose={() => setLoginError(null)} message={loginError} type="error" />;

  return (
    <>
      <div inert={loginVisible} style={{ display: 'contents' }}>
        <TabRoot manager={manager} />
        <GuidanceRoot />
        <DeckInputGuidance />
      </div>
      {loginVisible ? <AccountLoginPage onDismiss={() => userService.setLoginOpen(false)} /> : null}
      {loginErrorDialog}
    </>
  );
}

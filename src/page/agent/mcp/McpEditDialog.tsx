import { useEffect, useState } from 'react';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { Box, Dialog, Typography } from '@mui/material';
import { Footer } from '@/page/home/keys/edit/frame/Footer.tsx';
import type { AiMcpServer } from '@/common/model/AiMcpServer.ts';
import { Header } from '@/page/common/components/dialog/Header.tsx';
import { MCP_TRANSPORT } from '@/common/const/McpTransportConst.ts';
import { useBackDismiss } from '@/page/common/hooks/useBackDismiss.ts';
import { AlertDialog } from '@/common/components/dialog/AlertDialog.tsx';
import { EditField } from '@/page/common/components/input/EditField.tsx';
import { useAiMcpServerService } from '@/common/context/useAppHooks.tsx';
import { isEmpty, isNotEmpty, isNotNil } from '@/common/util/StringUtil.ts';
import { ConfirmDialog } from '@/common/components/dialog/ConfirmDialog.tsx';
import { SwitchField } from '@/page/common/components/input/SwitchField.tsx';
import { usePlatformNativeMobile } from '@/common/context/usePlatformMobile';
import { resolveSheetDialogProps } from '@/page/common/util/scopedDialogUtil.ts';
import { TextareaField } from '@/page/common/components/input/TextareaField.tsx';
import { BizError, resolveResultMessage } from '@/common/model/resp/ResultModel.ts';
import { parseMcpServerJson, serializeMcpServerJson } from '@/page/common/util/mcpJsonCodec.ts';
import { SelectField, type SelectOption } from '@/page/common/components/input/SelectField.tsx';

interface McpEditDialogProps {
  open: boolean;
  onClose: () => void;
  onSaved: () => void;
  server: AiMcpServer | null;
}

export function McpEditDialog(props: McpEditDialogProps) {
  const { t, tf } = useI18n();
  const [saving, setSaving] = useState(false);
  const nativeMobile = usePlatformNativeMobile();
  const aiMcpServerService = useAiMcpServerService();
  const { open, server, onClose, onSaved } = props;
  const [form, setForm] = useState<AiMcpServer | null>(null);
  const [jsonText, setJsonText] = useState('');
  const [jsonError, setJsonError] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [probeWarn, setProbeWarn] = useState<string | null>(null);

  useBackDismiss(open, onClose, 'mcp-edit');

  function createEmptyForm(): AiMcpServer {
    return {
      id: null,
      sort: 100,
      name: null,
      enabled: true,
      transport: nativeMobile ? MCP_TRANSPORT.HTTP : MCP_TRANSPORT.STDIO,
      command: null,
      args: null,
      env: null,
      url: null,
      headers: null,
      timeout_secs: null,
    };
  }

  function isEditMode(): boolean {
    if (server) {
      return true;
    }
    return false;
  }

  function resolveTitle(): string {
    if (isEditMode()) {
      return t('agent.mcp.editTitle');
    }
    return t('agent.mcp.createTitle');
  }

  function resolveTransportOptions(): SelectOption[] {
    const stdio: SelectOption = { value: String(MCP_TRANSPORT.STDIO), label: 'stdio' };
    const http: SelectOption = { value: String(MCP_TRANSPORT.HTTP), label: 'streamable http' };
    if (nativeMobile && form?.transport !== MCP_TRANSPORT.STDIO) {
      return [http];
    }
    return [stdio, http];
  }

  function applyForm(next: AiMcpServer): void {
    setForm(next);
    setJsonText(serializeMcpServerJson(next));
    setJsonError(null);
  }

  function canSave(): boolean {
    if (!form) {
      return false;
    }
    if (isNotNil(jsonError)) {
      return false;
    }
    if (isEmpty(form.name)) {
      return false;
    }
    if (form.transport === MCP_TRANSPORT.STDIO && isEmpty(form.command)) {
      return false;
    }
    if (form.transport === MCP_TRANSPORT.HTTP && isEmpty(form.url)) {
      return false;
    }
    return !saving;
  }

  function handleNameChange(value: string): void {
    if (!form) {
      return;
    }
    if (!value) {
      applyForm({ ...form, name: null });
      return;
    }
    applyForm({ ...form, name: value });
  }

  function handleTransportChange(value: string): void {
    if (!form) {
      return;
    }
    const transport = Number(value);
    if (transport === MCP_TRANSPORT.HTTP) {
      applyForm({
        ...form,
        transport,
        command: null,
        args: null,
        env: null,
      });
      return;
    }
    applyForm({
      ...form,
      transport,
      url: null,
      headers: null,
    });
  }

  function handleEnabledChange(checked: boolean): void {
    if (!form) {
      return;
    }
    applyForm({ ...form, enabled: checked });
  }

  function handleCommandChange(value: string): void {
    if (!form) {
      return;
    }
    if (!value) {
      applyForm({ ...form, command: null });
      return;
    }
    applyForm({ ...form, command: value });
  }

  function handleUrlChange(value: string): void {
    if (!form) {
      return;
    }
    if (!value) {
      applyForm({ ...form, url: null });
      return;
    }
    applyForm({ ...form, url: value });
  }

  function resolvePrettyJson(raw: string | null): string | null {
    if (!raw) {
      return null;
    }
    try {
      return JSON.stringify(JSON.parse(raw), null, 2);
    } catch {
      return raw;
    }
  }

  function handleArgsChange(value: string): void {
    if (!form) {
      return;
    }
    if (!value.trim()) {
      applyForm({ ...form, args: null });
      return;
    }
    try {
      const parsed: unknown = JSON.parse(value);
      if (!Array.isArray(parsed)) {
        setJsonError(t('agent.mcp.argsMustArray'));
        setForm({ ...form, args: value });
        return;
      }
      applyForm({ ...form, args: JSON.stringify(parsed) });
    } catch {
      setJsonError(t('agent.mcp.argsJsonInvalid'));
      setForm({ ...form, args: value });
    }
  }

  function handleEnvChange(value: string): void {
    if (!form) {
      return;
    }
    if (!value.trim()) {
      applyForm({ ...form, env: null });
      return;
    }
    try {
      const parsed: unknown = JSON.parse(value);
      if (!parsed || Array.isArray(parsed) || Object.getPrototypeOf(parsed) !== Object.prototype) {
        setJsonError(t('agent.mcp.envMustObject'));
        setForm({ ...form, env: value });
        return;
      }
      applyForm({ ...form, env: JSON.stringify(parsed) });
    } catch {
      setJsonError(t('agent.mcp.envJsonInvalid'));
      setForm({ ...form, env: value });
    }
  }

  function handleHeadersChange(value: string): void {
    if (!form) {
      return;
    }
    if (!value.trim()) {
      applyForm({ ...form, headers: null });
      return;
    }
    try {
      const parsed: unknown = JSON.parse(value);
      if (!parsed || Array.isArray(parsed) || Object.getPrototypeOf(parsed) !== Object.prototype) {
        setJsonError(t('agent.mcp.headersMustObject'));
        setForm({ ...form, headers: value });
        return;
      }
      applyForm({ ...form, headers: JSON.stringify(parsed) });
    } catch {
      setJsonError(t('agent.mcp.headersJsonInvalid'));
      setForm({ ...form, headers: value });
    }
  }

  function handleJsonChange(value: string): void {
    setJsonText(value);
    if (!form) {
      return;
    }
    const parsed = parseMcpServerJson(value, form);
    if (!parsed.ok) {
      setJsonError(parsed.message);
      return;
    }
    setJsonError(null);
    setForm(parsed.server);
  }

  function handleAlertClose(): void {
    setError(null);
  }

  function resolveProbeErrorMessage(e: unknown): string {
    if (e instanceof BizError) {
      return resolveResultMessage(e.code);
    }
    if (e instanceof Error) {
      return e.message;
    }
    return t('agent.mcp.probeFailed');
  }

  async function persistServer(payload: AiMcpServer): Promise<void> {
    if (isEditMode()) {
      await aiMcpServerService.update(payload);
      return;
    }
    await aiMcpServerService.insert(payload);
  }

  async function handleSave(): Promise<void> {
    if (!form) {
      return;
    }
    if (!canSave()) {
      return;
    }
    setSaving(true);
    setError(null);
    const payload: AiMcpServer = {
      ...form,
      name: form.name ? form.name.trim() : null,
    };
    let warn: string | null = null;
    if (payload.enabled) {
      try {
        await aiMcpServerService.probe(payload);
      } catch (e) {
        warn = tf('agent.mcp.probeSavedWarn', { error: resolveProbeErrorMessage(e) });
      }
    }
    try {
      await persistServer(payload);
    } catch (e) {
      if (e instanceof BizError) {
        setError(resolveResultMessage(e.code));
        return;
      }
      if (e instanceof Error) {
        setError(e.message);
        return;
      }
      setError(t('agent.mcp.saveFailed'));
      return;
    } finally {
      setSaving(false);
    }
    if (warn) {
      setProbeWarn(warn);
      return;
    }
    onSaved();
  }

  function handleProbeWarnConfirm(): void {
    setProbeWarn(null);
    onSaved();
  }

  useEffect(() => {
    if (!open) {
      return;
    }
    const next = server ? { ...server } : createEmptyForm();
    setForm(next);
    setJsonText(serializeMcpServerJson(next));
    setJsonError(null);
    setError(null);
    setProbeWarn(null);
  }, [open, server]);

  return (
    <Dialog open={open} onClose={onClose} {...resolveSheetDialogProps({ className: 'tm-edit-dialog-paper tm-mcp-edit-dialog-paper' })}>
      <Box className="tm-edit-dialog-body tm-mcp-edit-dialog-body">
        <Header title={resolveTitle()} onClose={onClose} />
        <Box className="tm-mcp-edit-split">
          <Box className="tm-mcp-edit-form">
            <EditField label={t('agent.mcp.name')} value={form ? form.name : null} onChange={handleNameChange} placeholder={t('agent.mcp.namePlaceholder')} autoFocus />
            <SelectField label={t('agent.mcp.transport')} value={form ? String(form.transport) : String(MCP_TRANSPORT.STDIO)} onChange={handleTransportChange} options={resolveTransportOptions()} />
            <SwitchField label={t('agent.mcp.enabled')} value={form ? form.enabled : false} onChange={handleEnabledChange} />
            <Box className="tm-mcp-edit-transport-slot">
              {form && form.transport === MCP_TRANSPORT.HTTP ? (
                <>
                  <EditField label="URL" value={form.url} onChange={handleUrlChange} placeholder="http://127.0.0.1:8080/mcp" />
                  <TextareaField label="Headers" value={resolvePrettyJson(form.headers)} onChange={handleHeadersChange} placeholder='{"Authorization":"Bearer xxx"}' minRows={2} maxRows={4} />
                </>
              ) : (
                <>
                  <EditField label={t('agent.mcp.command')} value={form ? form.command : null} onChange={handleCommandChange} placeholder={t('agent.mcp.commandPlaceholder')} />
                  <TextareaField label={t('agent.mcp.args')} value={resolvePrettyJson(form ? form.args : null)} onChange={handleArgsChange} placeholder='["/c","npx","-y","..."]' minRows={2} maxRows={5} />
                  <TextareaField label={t('agent.mcp.env')} value={resolvePrettyJson(form ? form.env : null)} onChange={handleEnvChange} placeholder='{"KEY":"VALUE"}' minRows={2} maxRows={4} />
                </>
              )}
            </Box>
          </Box>
          <Box className="tm-mcp-edit-json-pane">
            <Typography className="tm-mcp-edit-json-title">mcp.json</Typography>
            <textarea className="tm-mcp-edit-json-input" spellCheck={false} value={jsonText} onChange={event => handleJsonChange(event.target.value)} placeholder={'{\n  "chrome-devtools": {\n    "command": "cmd",\n    "args": ["/c", "npx", "-y", "chrome-devtools-mcp@latest"]\n  }\n}'} />
            {isNotEmpty(jsonError) ? <Typography className="tm-mcp-edit-json-error">{jsonError}</Typography> : <Typography className="tm-mcp-edit-json-ok">{t('agent.mcp.jsonSync')}</Typography>}
          </Box>
        </Box>
        <AlertDialog duration={2000} open={isNotEmpty(error)} handleClose={handleAlertClose} message={error} type="error" />
        <ConfirmDialog open={isNotEmpty(probeWarn)} title={t('agent.mcp.probeNotReady')} message={probeWarn ? probeWarn : ''} cancelLabel={t('common.close')} confirmLabel={t('agent.mcp.gotIt')} onCancel={handleProbeWarnConfirm} onConfirm={handleProbeWarnConfirm} />
        <Footer
          disable={!canSave()}
          saving={saving}
          onCancel={onClose}
          onSave={() => {
            void handleSave();
          }}
        />
      </Box>
    </Dialog>
  );
}

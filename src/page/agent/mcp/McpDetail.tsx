import { useEffect, useState } from 'react';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { Box, Dialog, Typography } from '@mui/material';
import { Header } from '@/page/common/components/dialog/Header.tsx';
import { useAiMcpServerService } from '@/common/context/useAppHooks.tsx';
import { resolveSheetDialogProps } from '@/page/common/util/scopedDialogUtil.ts';
import { isEmpty, isNotEmpty, isNil, isNotNil } from '@/common/util/StringUtil.ts';
import { BizError, resolveResultMessage } from '@/common/model/resp/ResultModel.ts';
import type { McpToolSnapshot, RespAiMcpServerRuntime } from '@/common/model/AiMcpServer.ts';

interface McpDetailProps {
  onClose: () => void;
  item: RespAiMcpServerRuntime;
}

export function McpDetail(props: McpDetailProps) {
  const { t, tf } = useI18n();
  const { item, onClose } = props;
  const [loading, setLoading] = useState(true);
  const aiMcpServerService = useAiMcpServerService();
  const [tools, setTools] = useState<McpToolSnapshot[] | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);

  async function loadTools(): Promise<void> {
    if (isEmpty(item.server.id)) {
      setTools(null);
      setLoadError(t('agent.mcp.invalidId'));
      setLoading(false);
      return;
    }
    if (!item.server.enabled) {
      setTools(null);
      setLoadError(null);
      setLoading(false);
      return;
    }
    setLoading(true);
    setLoadError(null);
    try {
      const res = await aiMcpServerService.listTools(item.server.id);
      setTools(res.tools);
    } catch (e) {
      setTools(null);
      if (e instanceof BizError) {
        setLoadError(resolveResultMessage(e.code));
      } else if (e instanceof Error) {
        setLoadError(e.message);
      } else {
        setLoadError(t('agent.mcp.loadToolsFailed'));
      }
    } finally {
      setLoading(false);
    }
  }

  function resolveStatusText(): string {
    if (!item.server.enabled) {
      return t('agent.mcp.disabled');
    }
    if (item.connected) {
      return tf('agent.mcp.toolCount', { n: item.tool_count });
    }
    return t('agent.mcp.notConnected');
  }

  function resolveTitle(): string {
    if (isNotEmpty(item.server.name)) {
      return item.server.name;
    }
    return t('agent.mcp.title');
  }

  function resolveErrorText(): string | null {
    if (!item.server.enabled) {
      return null;
    }
    if (isNotEmpty(item.last_error)) {
      return item.last_error;
    }
    if (isNotEmpty(loadError)) {
      return loadError;
    }
    return null;
  }

  useEffect(() => {
    void loadTools();
  }, [aiMcpServerService, item.server.id]);

  const errorText = resolveErrorText();

  return (
    <Dialog open onClose={onClose} {...resolveSheetDialogProps({ className: 'tm-edit-dialog-paper tm-mcp-detail-dialog-paper' })}>
      <Box className="tm-edit-dialog-body tm-mcp-detail">
        <Header title={resolveTitle()} onClose={onClose} />
        <Typography className="tm-mcp-detail-status">{resolveStatusText()}</Typography>
        {isNotNil(errorText) ? (
          <Box className="tm-mcp-detail-error">
            <Typography className="tm-mcp-detail-error-label">{t('agent.mcp.error')}</Typography>
            <Typography className="tm-mcp-detail-error-text">{errorText}</Typography>
          </Box>
        ) : null}
        <Box className="tm-mcp-detail-body">
          {loading ? <Typography className="tm-mcp-detail-empty">{t('agent.mcp.loadingTools')}</Typography> : null}
          {!loading && isNil(tools) ? <Typography className="tm-mcp-detail-empty">{t('agent.mcp.noToolInfo')}</Typography> : null}
          {!loading && isNotNil(tools) && tools.length === 0 ? <Typography className="tm-mcp-detail-empty">{t('agent.mcp.noTools')}</Typography> : null}
          {!loading && isNotNil(tools)
            ? tools.map(tool => (
                <Box key={tool.name} className="tm-mcp-tool-row">
                  <Typography className="tm-mcp-tool-name">{tool.name}</Typography>
                  {tool.description ? <Typography className="tm-mcp-tool-desc">{tool.description}</Typography> : <Typography className="tm-mcp-tool-desc is-empty">{t('agent.mcp.noDescription')}</Typography>}
                </Box>
              ))
            : null}
        </Box>
      </Box>
    </Dialog>
  );
}

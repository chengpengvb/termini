import type { MouseEvent } from 'react';
import Add from '@mui/icons-material/Add';
import { useEffect, useState } from 'react';
import { useI18n } from '@/common/i18n/useI18n.ts';
import MoreHoriz from '@mui/icons-material/MoreHoriz';
import { Footer } from '@/page/home/keys/edit/frame/Footer.tsx';
import { Box, ButtonBase, Dialog, Typography } from '@mui/material';
import { Header } from '@/page/common/components/dialog/Header.tsx';
import { usePlatformMobile } from '@/common/context/usePlatformMobile';
import { EditField } from '@/page/common/components/input/EditField.tsx';
import { isEmpty, isNotEmpty, isNotNil } from '@/common/util/StringUtil.ts';
import { resolveSheetDialogProps } from '@/page/common/util/scopedDialogUtil.ts';
import { BizError, resolveResultMessage } from '@/common/model/resp/ResultModel.ts';
import { useAiMcpServerService, useSystemUtilService } from '@/common/context/useAppHooks.tsx';
import { MCP_MARKET_RUNTIME, type McpMarketEnvKey, type McpMarketItem } from '@/common/model/AiMcpServer.ts';

interface McpMarketProps {
  keyword: string | null;
  installedNames: string[];
  onInstalled: () => void;
  onError: (message: string) => void;
  onInstalledMenu: (event: MouseEvent<HTMLElement>, name: string) => void;
}

const RUNTIME_LABEL: Record<number, string> = {
  [MCP_MARKET_RUNTIME.NODE]: 'Node',
  [MCP_MARKET_RUNTIME.PYTHON]: 'Python',
  [MCP_MARKET_RUNTIME.BINARY]: 'CLI',
  [MCP_MARKET_RUNTIME.HTTP]: 'HTTP',
};

function parseEnvKeys(raw: string | null): McpMarketEnvKey[] {
  if (isEmpty(raw)) {
    return [];
  }
  return JSON.parse(raw) as McpMarketEnvKey[];
}

function parseArgs(raw: string | null): string[] {
  if (isEmpty(raw)) {
    return [];
  }
  return JSON.parse(raw) as string[];
}

function resolveTitle(item: McpMarketItem): string {
  if (isNotEmpty(item.title)) {
    return item.title;
  }
  return item.name;
}

function resolveLetter(item: McpMarketItem): string {
  return resolveTitle(item).trim().charAt(0).toUpperCase();
}

function resolveLaunchText(item: McpMarketItem): string {
  const extra = parseArgs(item.args).join(' ');
  if (item.runtime === MCP_MARKET_RUNTIME.HTTP) {
    return isEmpty(item.url) ? '' : item.url;
  }
  if (item.runtime === MCP_MARKET_RUNTIME.NODE) {
    return `npx -y ${item.pkg} ${extra}`.trim();
  }
  if (item.runtime === MCP_MARKET_RUNTIME.PYTHON) {
    return `uvx ${item.pkg} ${extra}`.trim();
  }
  return `${item.pkg} ${extra}`.trim();
}

function matchKeyword(item: McpMarketItem, keyword: string | null): boolean {
  if (isEmpty(keyword)) {
    return true;
  }
  const needle = keyword.trim().toLowerCase();
  const haystack = [resolveTitle(item), item.name, item.description, item.category].filter(isNotEmpty).join(' ').toLowerCase();
  return haystack.includes(needle);
}

function groupByCategory(items: McpMarketItem[]): Array<{ category: string; items: McpMarketItem[] }> {
  const groups: Array<{ category: string; items: McpMarketItem[] }> = [];
  for (const item of items) {
    const category = isEmpty(item.category) ? '' : item.category;
    const group = groups.find(g => g.category === category);
    if (group) {
      group.items.push(item);
    } else {
      groups.push({ category, items: [item] });
    }
  }
  return groups;
}

export function McpMarket(props: McpMarketProps) {
  const { t } = useI18n();
  const mobileViewport = usePlatformMobile();
  const [loading, setLoading] = useState(true);
  const systemUtilService = useSystemUtilService();
  const aiMcpServerService = useAiMcpServerService();
  const [installing, setInstalling] = useState(false);
  const [items, setItems] = useState<McpMarketItem[]>([]);
  const [selected, setSelected] = useState<McpMarketItem | null>(null);
  const selectedKeys = selected ? parseEnvKeys(selected.envKeys) : [];
  const [envValues, setEnvValues] = useState<Record<string, string>>({});
  const { keyword, installedNames, onInstalled, onInstalledMenu, onError } = props;
  const groups = groupByCategory(items.filter(item => matchKeyword(item, keyword)));

  function isInstalled(item: McpMarketItem): boolean {
    return installedNames.includes(item.name);
  }

  function resolveErrorText(e: unknown): string {
    if (e instanceof BizError) {
      return resolveResultMessage(e.code);
    }
    if (e instanceof Error) {
      return e.message;
    }
    return t('agent.mcp.market.installFailed');
  }

  function resolveCategoryTitle(category: string): string {
    if (isEmpty(category)) {
      return t('agent.mcp.market.title');
    }
    const key = `agent.mcp.market.categories.${category}`;
    const text = t(key);
    if (text === key) {
      throw new Error(key);
    }
    return text;
  }

  async function install(item: McpMarketItem, env: Record<string, string>): Promise<void> {
    setInstalling(true);
    try {
      await aiMcpServerService.installMarket({ marketId: item.id, env });
      setSelected(null);
      onInstalled();
    } catch (e) {
      onError(resolveErrorText(e));
    } finally {
      setInstalling(false);
    }
  }

  function handleOpen(item: McpMarketItem): void {
    setEnvValues({});
    setSelected(item);
  }

  function handleQuickInstall(item: McpMarketItem): void {
    if (parseEnvKeys(item.envKeys).length > 0) {
      handleOpen(item);
      return;
    }
    void install(item, {});
  }

  function handleEnvChange(key: string, value: string): void {
    setEnvValues(current => ({ ...current, [key]: value }));
  }

  function canSubmit(): boolean {
    if (installing || !selected || isInstalled(selected)) {
      return false;
    }
    return selectedKeys.every(key => !key.required || isNotEmpty(envValues[key.key]));
  }

  function handleSubmit(): void {
    if (!selected || !canSubmit()) {
      return;
    }
    const env: Record<string, string> = {};
    for (const key of selectedKeys) {
      const value = envValues[key.key];
      if (isNotEmpty(value)) {
        env[key.key] = value.trim();
      }
    }
    void install(selected, env);
  }

  function handleCloseDialog(): void {
    if (installing) {
      return;
    }
    setSelected(null);
  }

  function handleOpenHomepage(url: string): void {
    systemUtilService.openUrl(url).catch(e => {
      onError(resolveErrorText(e));
    });
  }

  function resolveEnvLabel(key: McpMarketEnvKey): string {
    if (isNotEmpty(key.label)) {
      return key.label;
    }
    return key.key;
  }

  function renderIcon(item: McpMarketItem, className: string) {
    return (
      <Box className={className} aria-hidden>
        {isNotEmpty(item.icon) ? <img src={item.icon} alt="" className="tm-mcp-market-icon-img" /> : <span className="tm-mcp-market-icon-letter">{resolveLetter(item)}</span>}
      </Box>
    );
  }

  function renderRow(item: McpMarketItem) {
    const installed = isInstalled(item);
    return (
      <Box key={item.id} className="tm-agent-list-row" role="button" tabIndex={0} onClick={() => handleOpen(item)}>
        {renderIcon(item, 'tm-mcp-market-icon')}
        <Box className="tm-agent-list-row-main">
          <Typography className="tm-agent-list-row-name">{resolveTitle(item)}</Typography>
          {isNotEmpty(item.description) ? <Typography className="tm-agent-list-row-desc">{item.description}</Typography> : null}
        </Box>
        <ButtonBase
          disableRipple
          disabled={installing}
          aria-label={installed ? t('agent.mcp.more') : t('agent.mcp.market.install')}
          className="tm-mcp-market-action"
          onClick={event => {
            event.stopPropagation();
            if (installed) {
              onInstalledMenu(event, item.name);
              return;
            }
            handleQuickInstall(item);
          }}
        >
          {installed ? <MoreHoriz sx={{ fontSize: 18 }} /> : <Add sx={{ fontSize: 18 }} />}
        </ButtonBase>
      </Box>
    );
  }

  function renderDetail(item: McpMarketItem) {
    const launch = resolveLaunchText(item);
    return (
      <Box className="tm-mcp-market-detail">
        <Box className="tm-mcp-market-detail-head">
          {renderIcon(item, 'tm-mcp-market-icon is-large')}
          <Box className="tm-agent-list-row-main">
            <Typography className="tm-mcp-market-detail-title">{resolveTitle(item)}</Typography>
            <Typography className="tm-mcp-market-detail-name">{`${item.name} · ${RUNTIME_LABEL[item.runtime]}`}</Typography>
          </Box>
        </Box>
        {isNotEmpty(item.description) ? <Typography className="tm-mcp-market-detail-desc">{item.description}</Typography> : null}
        <Box className="tm-mcp-market-detail-rows">
          {isNotEmpty(item.category) ? (
            <Box className="tm-mcp-market-detail-row">
              <Typography className="tm-mcp-market-detail-label">{t('agent.mcp.market.category')}</Typography>
              <Typography className="tm-mcp-market-detail-code">{item.category}</Typography>
            </Box>
          ) : null}
          {isNotEmpty(launch) ? (
            <Box className="tm-mcp-market-detail-row">
              <Typography className="tm-mcp-market-detail-label">{t('agent.mcp.market.launch')}</Typography>
              <Typography className="tm-mcp-market-detail-code">{launch}</Typography>
            </Box>
          ) : null}
          {isNotEmpty(item.homepage) ? (
            <Box className="tm-mcp-market-detail-row">
              <Typography className="tm-mcp-market-detail-label">{t('agent.mcp.market.homepage')}</Typography>
              <ButtonBase disableRipple className="tm-mcp-market-detail-link" onClick={() => handleOpenHomepage(item.homepage as string)}>
                {item.homepage}
              </ButtonBase>
            </Box>
          ) : null}
        </Box>
        {selectedKeys.length > 0 ? (
          <Box className="tm-mcp-market-env-form">
            <Typography className="tm-mcp-market-env-hint">{isInstalled(item) ? t('agent.mcp.market.envInstalledHint') : t('agent.mcp.market.envHint')}</Typography>
            {isInstalled(item) ? null : selectedKeys.map(key => <EditField key={key.key} label={resolveEnvLabel(key)} labelWidth={150} value={envValues[key.key] ?? null} onChange={value => handleEnvChange(key.key, value)} placeholder={key.placeholder ?? key.key} />)}
          </Box>
        ) : null}
      </Box>
    );
  }

  useEffect(() => {
    let disposed = false;
    setLoading(true);
    aiMcpServerService
      .listMarket()
      .then(list => {
        if (!disposed) {
          setItems(list);
        }
      })
      .catch(e => {
        if (!disposed) {
          onError(resolveErrorText(e));
        }
      })
      .finally(() => {
        if (!disposed) {
          setLoading(false);
        }
      });
    return () => {
      disposed = true;
    };
  }, [aiMcpServerService]);

  if (loading) {
    return <Typography className="tm-home-loading-text">{t('agent.mcp.market.loading')}</Typography>;
  }

  return (
    <>
      {groups.map(group => (
        <Box key={group.category} className="tm-agent-section">
          <Box className="tm-agent-section-head">
            <Typography className={mobileViewport ? 'tm-home-section-label' : 'tm-agent-section-title'}>{resolveCategoryTitle(group.category)}</Typography>
          </Box>
          <Box className="tm-mcp-market-grid">{group.items.map(renderRow)}</Box>
        </Box>
      ))}
      <Dialog open={isNotNil(selected)} onClose={handleCloseDialog} {...resolveSheetDialogProps({ className: 'tm-edit-dialog-paper tm-mcp-market-dialog-paper' })}>
        <Box className="tm-edit-dialog-body">
          <Header title={t('agent.mcp.market.detailTitle')} onClose={handleCloseDialog} />
          {selected ? renderDetail(selected) : null}
          <Footer disable={!canSubmit()} saving={installing} saveLabel={selected && isInstalled(selected) ? t('agent.mcp.market.installed') : t('agent.mcp.market.install')} onCancel={handleCloseDialog} onSave={handleSubmit} />
        </Box>
      </Dialog>
    </>
  );
}

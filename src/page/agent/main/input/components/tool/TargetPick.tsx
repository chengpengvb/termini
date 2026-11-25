import type { ReactNode } from 'react';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { useEffect, useRef, useState } from 'react';
import type { Device } from '@/common/model/Device.ts';
import { LayersOutlined, Search } from '@mui/icons-material';
import type { RespTree } from '@/common/model/resp/RespTree.ts';
import { CHAT_TARGET_TYPE } from '@/common/const/ChatTargetConst.ts';
import { Box, ButtonBase, InputBase, Typography } from '@mui/material';
import { isEmpty, isNil, isNotEmpty } from '@/common/util/StringUtil.ts';
import type { ChatReference } from '@/common/model/dto/ChatReference.ts';
import { resolveDeviceOsIconBadged } from '@/page/common/util/deviceUtil.ts';

interface TargetPickProps {
  kind: number;
  tree: RespTree[];
  onPick: (ref: ChatReference) => void;
}

export function TargetPick(props: TargetPickProps) {
  const { t } = useI18n();
  const { kind, tree, onPick } = props;
  const searchInputRef = useRef<HTMLInputElement | null>(null);
  const [keyword, setKeyword] = useState<string | null>(null);

  function collectNodes(source: RespTree[], acc: RespTree[]): void {
    for (const node of source) {
      if (node.type === kind) {
        acc.push(node);
      }
      if (isNil(node.children)) {
        continue;
      }
      collectNodes(node.children, acc);
    }
  }

  function resolveNodes(): RespTree[] {
    const acc: RespTree[] = [];
    collectNodes(tree, acc);
    return acc;
  }

  function resolveIcon(node: RespTree): ReactNode {
    if (kind === CHAT_TARGET_TYPE.GROUP) {
      return <LayersOutlined sx={{ fontSize: 16 }} />;
    }
    if (isNil(node.data)) {
      return null;
    }
    const device: Device = JSON.parse(node.data);
    return resolveDeviceOsIconBadged(device.os_type, 'tm-agent-mount-item-icon-img');
  }

  function buildReference(node: RespTree): ChatReference {
    if (isNil(node.data)) {
      return { kind: node.type, id: node.id, title: node.title, data: null };
    }
    return { kind: node.type, id: node.id, title: node.title, data: JSON.parse(node.data) };
  }

  function resolveKeywordValue(): string {
    if (isNotEmpty(keyword)) {
      return keyword;
    }
    return '';
  }

  function resolveSearchPlaceholder(): string {
    if (kind === CHAT_TARGET_TYPE.GROUP) {
      return t('agent.mount.searchGroup');
    }
    return t('agent.mount.searchDevice');
  }

  function resolveFilteredNodes(): RespTree[] {
    const titled = resolveNodes().filter(node => isNotEmpty(node.title));
    if (isEmpty(keyword)) {
      return titled;
    }
    const value = keyword.toLowerCase();
    return titled.filter(node => {
      if (node.title.toLowerCase().includes(value)) {
        return true;
      }
      return node.id.toLowerCase().includes(value);
    });
  }

  function handleKeywordChange(raw: string): void {
    if (isEmpty(raw)) {
      setKeyword(null);
      return;
    }
    setKeyword(raw);
  }

  function handlePick(node: RespTree): void {
    onPick(buildReference(node));
  }

  useEffect(() => {
    setKeyword(null);
    searchInputRef.current?.focus({ preventScroll: true });
  }, [kind]);

  const nodes = resolveNodes();
  const filtered = resolveFilteredNodes();

  return (
    <Box className="tm-agent-mount-picker">
      <Box className="tm-agent-mount-search-row">
        <Search className="tm-agent-mount-search-icon" />
        <InputBase inputRef={searchInputRef} value={resolveKeywordValue()} placeholder={resolveSearchPlaceholder()} onChange={e => handleKeywordChange(e.target.value)} className="tm-agent-mount-search" />
      </Box>
      <Box className="tm-agent-mount-list">
        {nodes.length === 0 ? (
          <Typography className="tm-agent-mount-empty">{t('agent.mount.noData')}</Typography>
        ) : filtered.length === 0 ? (
          <Typography className="tm-agent-mount-empty">{t('agent.mount.noMatch')}</Typography>
        ) : (
          filtered.map(node => (
            <ButtonBase disableRipple key={node.id} onClick={() => handlePick(node)} className="tm-agent-mount-item">
              <Box className="tm-agent-mount-item-icon">{resolveIcon(node)}</Box>
              <Typography className="tm-agent-mount-item-name">{node.title}</Typography>
            </ButtonBase>
          ))
        )}
      </Box>
    </Box>
  );
}

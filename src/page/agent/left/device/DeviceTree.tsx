import { useEffect, useState } from 'react';
import { useI18n } from '@/common/i18n/useI18n.ts';
import type { RespTree } from '@/common/model/resp/RespTree.ts';
import { EditPanel } from '@/page/home/devices/edit/EditPanel.tsx';
import { Box, Collapse, InputBase, Typography } from '@mui/material';
import { CHAT_TARGET_TYPE } from '@/common/const/ChatTargetConst.ts';
import { ChevronRight, DnsOutlined, Search } from '@mui/icons-material';
import { resolveDeviceTabIcon } from '@/page/common/util/deviceUtil.ts';
import { isEmpty, isNil, isNotEmpty } from '@/common/util/StringUtil.ts';
import { NavMenuItem } from '@/page/common/components/nav/NavMenuItem.tsx';
import { DeviceTreeNode } from '@/page/agent/left/device/DeviceTreeNode.tsx';
import { SelectProtocol } from '@/page/protocol/common/select/SelectProtocol.tsx';
import { createEmptyReqDevice, type ReqDevice } from '@/common/model/req/ReqDevice.ts';
import { useDeviceEditService, useDeviceService, useTabManagerService } from '@/common/context/useAppHooks.tsx';

export function DeviceTree(props: { selected: boolean }) {
  const { t } = useI18n();
  const { selected } = props;
  const deviceService = useDeviceService();
  const [expanded, setExpanded] = useState(true);
  const deviceEditService = useDeviceEditService();
  const tabManagerService = useTabManagerService();
  const [tree, setTree] = useState<RespTree[]>([]);
  const [drawerVisible, setDrawerVisible] = useState(false);
  const [keyword, setKeyword] = useState<string | null>(null);
  const [expandedIds, setExpandedIds] = useState<Record<string, boolean>>({});
  const [reqDevice, setReqDevice] = useState<ReqDevice>(createEmptyReqDevice());
  const [selectedAction, setSelectedAction] = useState<number>(() => deviceEditService.getSelectedAction());

  const handleHeaderClick = (): void => {
    setExpanded(prev => !prev);
  };

  const collectGroupExpanded = (nodes: RespTree[], prev: Record<string, boolean>, next: Record<string, boolean>): void => {
    for (const node of nodes) {
      if (node.type === CHAT_TARGET_TYPE.GROUP) {
        if (isNil(prev[node.id])) {
          next[node.id] = true;
        } else {
          next[node.id] = prev[node.id];
        }
      }
      if (!node.children) {
        continue;
      }
      collectGroupExpanded(node.children, prev, next);
    }
  };

  const resolveExpandedIds = (nodes: RespTree[], prev: Record<string, boolean>): Record<string, boolean> => {
    const next: Record<string, boolean> = {};
    collectGroupExpanded(nodes, prev, next);
    return next;
  };

  const canRenderRootGroup = (node: RespTree): boolean => {
    if (node.type !== CHAT_TARGET_TYPE.GROUP) {
      return true;
    }
    if (!node.children) {
      return false;
    }
    return node.children.length > 0;
  };

  const resolveVisibleTree = (nodes: RespTree[]): RespTree[] => {
    return nodes.filter(node => canRenderRootGroup(node));
  };

  const updateDeviceList = async (): Promise<void> => {
    try {
      const nextTree = await deviceService.selectDeviceTree();
      const visibleTree = resolveVisibleTree(nextTree);
      setTree(visibleTree);
      setExpandedIds(prev => resolveExpandedIds(visibleTree, prev));
    } catch (error) {
      console.error('Failed to update device tree:', error);
    }
  };

  const handleToggleGroup = (groupId: string): void => {
    setExpandedIds(prev => {
      const next: Record<string, boolean> = { ...prev };
      next[groupId] = !prev[groupId];
      return next;
    });
  };

  const handleKeywordChange = (raw: string): void => {
    if (isEmpty(raw)) {
      setKeyword(null);
      return;
    }
    setKeyword(raw);
  };

  const resolveKeywordValue = (): string => {
    if (isNotEmpty(keyword)) {
      return keyword;
    }
    return '';
  };

  const filterNodes = (nodes: RespTree[], value: string): RespTree[] => {
    const out: RespTree[] = [];
    for (const node of nodes) {
      if (node.title.toLowerCase().includes(value)) {
        out.push(node);
        continue;
      }
      if (node.type !== CHAT_TARGET_TYPE.GROUP) {
        continue;
      }
      if (!node.children) {
        continue;
      }
      const children = filterNodes(node.children, value);
      if (children.length === 0) {
        continue;
      }
      out.push({ ...node, children });
    }
    return out;
  };

  const resolveDisplayTree = (): RespTree[] => {
    if (isEmpty(keyword)) {
      return tree;
    }
    return filterNodes(tree, keyword.toLowerCase());
  };

  const collectSearchExpanded = (nodes: RespTree[], next: Record<string, boolean>): void => {
    for (const node of nodes) {
      if (node.type === CHAT_TARGET_TYPE.GROUP) {
        next[node.id] = true;
      }
      if (!node.children) {
        continue;
      }
      collectSearchExpanded(node.children, next);
    }
  };

  const resolveDisplayExpandedIds = (displayTree: RespTree[]): Record<string, boolean> => {
    if (isEmpty(keyword)) {
      return expandedIds;
    }
    const next: Record<string, boolean> = {};
    collectSearchExpanded(displayTree, next);
    return next;
  };

  const handleOpenDevice = (value: ReqDevice): void => {
    const draft = deviceEditService.getDraft(value.device.id);
    if (draft === null) {
      setReqDevice(value);
      return;
    }
    setReqDevice(draft);
  };

  const handleClose = (): void => {
    setDrawerVisible(false);
  };

  const handleCancel = (): void => {
    deviceEditService.clearDraft(reqDevice.device.id);
    setDrawerVisible(false);
  };

  const handleDraftChange = (next: ReqDevice): void => {
    deviceEditService.setDraft(next);
  };

  const handleSelectedActionChange = (action: number): void => {
    deviceEditService.setSelectedAction(action);
  };

  const handleSave = (_value: ReqDevice): void => {
    deviceEditService.clearDraft(reqDevice.device.id);
    setDrawerVisible(false);
  };

  const handleConnect = (value: ReqDevice): void => {
    deviceEditService.clearDraft(reqDevice.device.id);
    tabManagerService.openTab({
      panes: [
        {
          tabBar: {
            canClose: true,
            title: value.device.name,
            icon: resolveDeviceTabIcon(value.device.os_type),
          },
          content: <SelectProtocol reqDevice={value} />,
        },
      ],
    });
    setDrawerVisible(false);
  };

  useEffect(() => {
    updateDeviceList().then();
    const unsubscribe = deviceService.addDeviceListChangeListener(() => {
      updateDeviceList().then();
    });
    return () => {
      unsubscribe();
    };
  }, [deviceService]);

  useEffect(() => {
    if (!selected) {
      return;
    }
    void updateDeviceList();
  }, [selected]);

  useEffect(() => {
    const offAction = deviceEditService.addSelectedActionListener(setSelectedAction);
    return () => {
      offAction();
    };
  }, [deviceEditService]);

  const displayTree = resolveDisplayTree();
  const displayExpandedIds = resolveDisplayExpandedIds(displayTree);

  return (
    <>
      <Box className={expanded ? 'tm-agent-left-section is-device' : 'tm-agent-left-section is-device is-collapsed'}>
        <Box className="tm-agent-left-section-nav tm-nav-menu">
          <NavMenuItem label={t('agent.left.deviceRefs')} selected={selected} icon={<DnsOutlined className="tm-nav-item-icon" />} endAdornment={<ChevronRight className={expanded ? 'tm-nav-item-chevron is-expanded' : 'tm-nav-item-chevron'} />} onClick={handleHeaderClick} />
        </Box>
        <Collapse in={expanded} timeout={280} sx={{ transitionTimingFunction: 'var(--tm-easing-ios)' }} className={expanded ? 'tm-agent-left-section-collapse is-device-fill' : 'tm-agent-left-section-collapse'}>
          <Box className="tm-agent-left-section-body is-device-tree">
            {tree.length === 0 ? (
              <Typography className="tm-agent-left-placeholder">{t('agent.left.noDevices')}</Typography>
            ) : (
              <>
                <Box className="tm-agent-mount-search-row">
                  <Search className="tm-agent-mount-search-icon" />
                  <InputBase value={resolveKeywordValue()} placeholder={t('agent.mount.searchDevice')} onChange={e => handleKeywordChange(e.target.value)} className="tm-agent-mount-search" />
                </Box>
                {displayTree.length === 0 ? (
                  <Typography className="tm-agent-left-placeholder">{t('agent.mount.noMatch')}</Typography>
                ) : (
                  displayTree.map(node => <DeviceTreeNode key={node.id} node={node} expandedIds={displayExpandedIds} onToggleGroup={handleToggleGroup} setReqDevice={handleOpenDevice} setDrawerVisible={setDrawerVisible} />)
                )}
              </>
            )}
          </Box>
        </Collapse>
      </Box>
      <EditPanel open={drawerVisible} reqDevice={reqDevice} selectedAction={selectedAction} onSelectedActionChange={handleSelectedActionChange} onChange={handleDraftChange} onSave={handleSave} onConnect={handleConnect} onClose={handleClose} onCancel={handleCancel} />
    </>
  );
}

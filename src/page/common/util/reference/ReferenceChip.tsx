import { Box } from '@mui/material';
import { DEVICE_OS_TYPE } from '@/common/model/Device.ts';
import { resolveGroupName } from '@/common/model/Group.ts';
import { isNil, isNotNil } from '@/common/util/StringUtil.ts';
import { CHAT_TARGET_TYPE } from '@/common/const/ChatTargetConst.ts';
import type { ChatReference } from '@/common/model/dto/ChatReference.ts';
import { resolveDeviceOsIconBadged } from '@/page/common/util/deviceUtil.ts';
import { resolveLocalPathName, resolveMcpPromptName } from '@/page/common/util/reference/token.ts';
import { useEffect, useState, type PointerEvent as ReactPointerEvent, type ReactNode } from 'react';
import { useAiMcpServerService, useAiSkillService, useDeviceService, useGroupService } from '@/common/context/useAppHooks.tsx';
import { Close, FolderOutlined, InsertDriveFileOutlined, BuildOutlined, HubOutlined, ExtensionOutlined } from '@mui/icons-material';

interface ReferenceChipProps {
  kind: number;
  id: string;
  interactive: boolean;
  onRemove: (() => void) | null;
}

export function ReferenceChip(props: ReferenceChipProps): ReactNode {
  const { kind, id, interactive, onRemove } = props;
  const deviceService = useDeviceService();
  const groupService = useGroupService();
  const aiSkillService = useAiSkillService();
  const aiMcpServerService = useAiMcpServerService();
  const chipClass: Record<number, string> = {
    [CHAT_TARGET_TYPE.DEVICE]: 'tm-agent-input-ref-chip is-device',
    [CHAT_TARGET_TYPE.GROUP]: 'tm-agent-input-ref-chip is-group',
    [CHAT_TARGET_TYPE.LOCAL_PATH]: 'tm-agent-input-ref-chip is-localpath',
    [CHAT_TARGET_TYPE.SKILL]: 'tm-agent-input-ref-chip is-skill',
    [CHAT_TARGET_TYPE.MCP_PROMPT]: 'tm-agent-input-ref-chip is-mcpprompt',
    [CHAT_TARGET_TYPE.MCP_TOOL]: 'tm-agent-input-ref-chip is-mcptool',
    [CHAT_TARGET_TYPE.MCP_SERVER]: 'tm-agent-input-ref-chip is-mcpserver',
  };
  const [resolved, setResolved] = useState<ChatReference>({ kind, id, title: id, data: null });

  function resolveDeviceOsType(): number {
    if (!resolved.data) {
      return DEVICE_OS_TYPE.UNKNOWN;
    }
    const osType = resolved.data.os_type;
    if (!isNotNil(osType)) {
      return DEVICE_OS_TYPE.UNKNOWN;
    }
    return osType;
  }

  function resolveLocalPathIcon(): ReactNode {
    if (resolved.title.includes('.')) {
      return <InsertDriveFileOutlined className="tm-agent-input-ref-chip-icon-svg" />;
    }
    return <FolderOutlined className="tm-agent-input-ref-chip-icon-svg" />;
  }

  function resolveChipIcon(): ReactNode {
    if (resolved.kind === CHAT_TARGET_TYPE.DEVICE) {
      return resolveDeviceOsIconBadged(resolveDeviceOsType(), 'tm-agent-input-ref-chip-icon-svg');
    }
    if (resolved.kind === CHAT_TARGET_TYPE.LOCAL_PATH) {
      return resolveLocalPathIcon();
    }
    if (resolved.kind === CHAT_TARGET_TYPE.SKILL) {
      return <BuildOutlined className="tm-agent-input-ref-chip-icon-svg" />;
    }
    if (resolved.kind === CHAT_TARGET_TYPE.MCP_PROMPT || resolved.kind === CHAT_TARGET_TYPE.MCP_SERVER) {
      return <HubOutlined className="tm-agent-input-ref-chip-icon-svg" />;
    }
    if (resolved.kind === CHAT_TARGET_TYPE.MCP_TOOL) {
      return <ExtensionOutlined className="tm-agent-input-ref-chip-icon-svg" />;
    }
    return null;
  }

  function handleClosePointerDown(e: ReactPointerEvent<HTMLElement>): void {
    e.preventDefault();
    e.stopPropagation();
    if (!onRemove) {
      return;
    }
    onRemove();
  }

  useEffect(() => {
    let cancelled = false;
    if (kind === CHAT_TARGET_TYPE.LOCAL_PATH) {
      setResolved({ kind, id, title: resolveLocalPathName(id), data: null });
      return;
    }
    if (kind === CHAT_TARGET_TYPE.MCP_PROMPT || kind === CHAT_TARGET_TYPE.MCP_TOOL) {
      setResolved({ kind, id, title: resolveMcpPromptName(id), data: null });
      return;
    }
    setResolved({ kind, id, title: id, data: null });
    if (kind === CHAT_TARGET_TYPE.DEVICE) {
      deviceService.selectById(id).then(device => {
        if (cancelled) {
          return;
        }
        if (isNil(device)) {
          return;
        }
        setResolved({ kind, id, title: device.name, data: device });
      });
      return () => {
        cancelled = true;
      };
    }
    if (kind === CHAT_TARGET_TYPE.GROUP) {
      groupService.selectById(id).then(group => {
        if (cancelled) {
          return;
        }
        if (isNil(group)) {
          return;
        }
        setResolved({ kind, id, title: resolveGroupName(group), data: null });
      });
      return () => {
        cancelled = true;
      };
    }
    if (kind === CHAT_TARGET_TYPE.SKILL) {
      aiSkillService.selectById(id).then(skill => {
        if (cancelled) {
          return;
        }
        if (isNil(skill)) {
          return;
        }
        if (isNil(skill.name)) {
          return;
        }
        setResolved({ kind, id, title: skill.name, data: null });
      });
      return () => {
        cancelled = true;
      };
    }
    if (kind === CHAT_TARGET_TYPE.MCP_SERVER) {
      aiMcpServerService.selectById(id).then(server => {
        if (cancelled) {
          return;
        }
        if (isNil(server)) {
          return;
        }
        if (isNil(server.name)) {
          return;
        }
        setResolved({ kind, id, title: server.name, data: null });
      });
      return () => {
        cancelled = true;
      };
    }
    return () => {
      cancelled = true;
    };
  }, [kind, id, deviceService, groupService, aiSkillService, aiMcpServerService]);

  const icon = resolveChipIcon();
  const closeButton = interactive ? (
    <Box component="span" className="tm-agent-input-ref-chip-close" onPointerDown={handleClosePointerDown} contentEditable={false} data-close="1">
      <Close fontSize="inherit" />
    </Box>
  ) : null;

  return (
    <Box component="span" className={chipClass[resolved.kind]} contentEditable={false} data-reference="1" data-kind={resolved.kind} data-id={resolved.id}>
      {isNotNil(icon) ? (
        <Box component="span" className="tm-agent-input-ref-chip-icon-slot" contentEditable={false}>
          <Box component="span" className="tm-agent-input-ref-chip-icon" aria-hidden>
            {icon}
          </Box>
          {closeButton}
        </Box>
      ) : (
        closeButton
      )}
      <Box component="span" className="tm-agent-input-ref-chip-label">
        {resolved.title}
      </Box>
    </Box>
  );
}

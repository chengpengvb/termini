import { Box } from '@mui/material';
import { isNil, isNotNil } from '@/common/util/StringUtil.ts';
import type { AiChatItem } from '@/common/model/AiChatItem.ts';
import { ItemMcp } from '@/page/agent/main/body/tool/ItemMcp.tsx';
import { ItemTask } from '@/page/agent/main/body/tool/ItemTask.tsx';
import { ItemText } from '@/page/agent/main/body/tool/ItemText.tsx';
import { useItemHandle } from '@/page/common/hooks/useItemHandle.ts';
import { ItemFtpFs } from '@/page/agent/main/body/tool/ItemFtpFs.tsx';
import { ItemShell } from '@/page/agent/main/body/tool/ItemShell.tsx';
import { ItemSftpFs } from '@/page/agent/main/body/tool/ItemSftpFs.tsx';
import type { ItemHandle } from '@/page/common/dto/agent/ItemHandle.ts';
import { ItemAskUser } from '@/page/agent/main/body/tool/ItemAskUser.tsx';
import { ItemCodeMap } from '@/page/agent/main/body/tool/ItemCodeMap.tsx';
import { Ref, useCallback, useRef, useState, type ReactNode } from 'react';
import { AI_TOOL_ID, resolveAiToolId } from '@/common/const/AiToolConst.ts';
import { ItemFileRead } from '@/page/agent/main/body/tool/ItemFileRead.tsx';
import { ItemMailSend } from '@/page/agent/main/body/tool/ItemMailSend.tsx';
import { ItemWebFetch } from '@/page/agent/main/body/tool/ItemWebFetch.tsx';
import { ItemImageRead } from '@/page/agent/main/body/tool/ItemImageRead.tsx';
import { ItemScriptRun } from '@/page/agent/main/body/tool/ItemScriptRun.tsx';
import { ItemTodoWrite } from '@/page/agent/main/body/tool/ItemTodoWrite.tsx';
import { ItemWebSearch } from '@/page/agent/main/body/tool/ItemWebSearch.tsx';
import { RESP_EVENT_AGENT } from '@/common/model/resp/event/RespEventAgent.ts';
import type { AgentQuestionAnswer } from '@/common/model/req/event/WsAgent.ts';
import { ItemCodeSearch } from '@/page/agent/main/body/tool/ItemCodeSearch.tsx';
import { ItemCodeSymbol } from '@/page/agent/main/body/tool/ItemCodeSymbol.tsx';
import { ItemSkillWrite } from '@/page/agent/main/body/tool/ItemSkillWrite.tsx';
import { ItemCodeSemantic } from '@/page/agent/main/body/tool/ItemCodeSemantic.tsx';
import { ItemJobIndicator } from '@/page/agent/main/body/tool/ItemJobIndicator.tsx';
import { ItemSftpFileRead } from '@/page/agent/main/body/tool/ItemSftpFileRead.tsx';
import { ItemDesktopAction } from '@/page/agent/main/body/tool/ItemDesktopAction.tsx';
import { ItemDeviceProfile } from '@/page/agent/main/body/tool/ItemDeviceProfile.tsx';
import { ItemDocumentWrite } from '@/page/agent/main/body/tool/ItemDocumentWrite.tsx';
import { ItemImageGenerate } from '@/page/agent/main/body/tool/ItemImageGenerate.tsx';
import { ItemSearchReplace } from '@/page/agent/main/body/tool/ItemSearchReplace.tsx';
import { ItemSshShellBatch } from '@/page/agent/main/body/tool/ItemSshShellBatch.tsx';
import { ItemFtpTransferBatch } from '@/page/agent/main/body/tool/ItemFtpTransferBatch.tsx';
import { ItemTelnetShellBatch } from '@/page/agent/main/body/tool/ItemTelnetShellBatch.tsx';
import { ItemSftpSearchReplace } from '@/page/agent/main/body/tool/ItemSftpSearchReplace.tsx';
import { ItemSftpTransferBatch } from '@/page/agent/main/body/tool/ItemSftpTransferBatch.tsx';
import { ItemShellProcessKill, ItemShellProcessList, ItemShellProcessLog, ItemShellProcessWrite } from '@/page/agent/main/body/tool/ItemShellProcess.tsx';

interface ItemToolsProps {
  itemId: string;
  sessionId: string;
  ref?: Ref<ItemHandle>;
  onAskAnswer?: (answer: AgentQuestionAnswer) => void;
}

interface ToolChildProps {
  itemId: string;
  sessionId: string;
  ref?: Ref<ItemHandle>;
  onAskAnswer?: (answer: AgentQuestionAnswer) => void;
}

const TOOL_VIEW: Record<number, (props: ToolChildProps) => ReactNode> = {
  [AI_TOOL_ID.GROUP_LIST]: ItemText,
  [AI_TOOL_ID.DEVICE_LIST]: ItemText,
  [AI_TOOL_ID.DEVICE_IMPORT]: ItemText,
  [AI_TOOL_ID.SKILL_LIST]: ItemText,
  [AI_TOOL_ID.CODE_MAP]: ItemCodeMap,
  [AI_TOOL_ID.CHAT_HISTORY]: ItemText,
  [AI_TOOL_ID.LOCAL_SHELL]: ItemShell,
  [AI_TOOL_ID.SHELL_READONLY]: ItemShell,
  [AI_TOOL_ID.SHELL_PROCESS_LIST]: ItemShellProcessList,
  [AI_TOOL_ID.SHELL_PROCESS_LOG]: ItemShellProcessLog,
  [AI_TOOL_ID.SHELL_PROCESS_KILL]: ItemShellProcessKill,
  [AI_TOOL_ID.SHELL_PROCESS_WRITE]: ItemShellProcessWrite,
  [AI_TOOL_ID.TOOL_SEARCH]: ItemText,
  [AI_TOOL_ID.MAIL_SEND]: ItemMailSend,
  [AI_TOOL_ID.ENTITY_LIST]: ItemText,
  [AI_TOOL_ID.ENTITY_DESCRIBE]: ItemText,
  [AI_TOOL_ID.ENTITY_CALL]: ItemText,
  [AI_TOOL_ID.DERIVED_LIST]: ItemText,
  [AI_TOOL_ID.FILE_READ]: ItemFileRead,
  [AI_TOOL_ID.WEB_FETCH]: ItemWebFetch,
  [AI_TOOL_ID.WEB_SEARCH]: ItemWebSearch,
  [AI_TOOL_ID.CODE_SYMBOL]: ItemCodeSymbol,
  [AI_TOOL_ID.CODE_SEARCH]: ItemCodeSearch,
  [AI_TOOL_ID.CODE_SEMANTIC]: ItemCodeSemantic,
  [AI_TOOL_ID.TASK_LIST]: ItemText,
  [AI_TOOL_ID.TASK_LOG]: ItemText,
  [AI_TOOL_ID.TASK_KILL]: ItemText,
  [AI_TOOL_ID.TASK_WAIT]: ItemText,
  [AI_TOOL_ID.LOOP_START]: ItemText,
  [AI_TOOL_ID.LOOP_STOP]: ItemText,
  [AI_TOOL_ID.OUTPUT_READ]: ItemText,
  [AI_TOOL_ID.JOB_INDICATOR_DRAFT]: ItemText,
  [AI_TOOL_ID.JOB_INDICATOR_LIST]: ItemText,
  [AI_TOOL_ID.JOB_INDICATOR_CREATE]: ItemJobIndicator,
  [AI_TOOL_ID.JOB_INDICATOR_UPDATE]: ItemJobIndicator,
  [AI_TOOL_ID.SFTP_FILE_READ]: ItemSftpFileRead,
  [AI_TOOL_ID.SEARCH_REPLACE]: ItemSearchReplace,
  [AI_TOOL_ID.SSH_SHELL_BATCH]: ItemSshShellBatch,
  [AI_TOOL_ID.TELNET_SHELL_BATCH]: ItemTelnetShellBatch,
  [AI_TOOL_ID.SFTP_TRANSFER_BATCH]: ItemSftpTransferBatch,
  [AI_TOOL_ID.FTP_TRANSFER_BATCH]: ItemFtpTransferBatch,
  [AI_TOOL_ID.SFTP_FS]: ItemSftpFs,
  [AI_TOOL_ID.FTP_FS]: ItemFtpFs,
  [AI_TOOL_ID.SFTP_SEARCH_REPLACE]: ItemSftpSearchReplace,
  [AI_TOOL_ID.PDF_READ]: ItemText,
  [AI_TOOL_ID.XLSX_READ]: ItemText,
  [AI_TOOL_ID.XLSX_WRITE]: ItemDocumentWrite,
  [AI_TOOL_ID.DOCX_READ]: ItemText,
  [AI_TOOL_ID.DOCX_WRITE]: ItemDocumentWrite,
  [AI_TOOL_ID.PDF_WRITE]: ItemDocumentWrite,
  [AI_TOOL_ID.TODO_WRITE]: ItemTodoWrite,
  [AI_TOOL_ID.TASK]: ItemTask,
  [AI_TOOL_ID.SKILL_CREATE]: ItemSkillWrite,
  [AI_TOOL_ID.SKILL_UPDATE]: ItemSkillWrite,
  [AI_TOOL_ID.SNMP_PROBE]: ItemText,
  [AI_TOOL_ID.DEVICE_PROFILE_UPDATE]: ItemDeviceProfile,
  [AI_TOOL_ID.ASK_USER]: ItemAskUser,
  [AI_TOOL_ID.IMAGE_READ]: ItemImageRead,
  [AI_TOOL_ID.IMAGE_GENERATE]: ItemImageGenerate,
  [AI_TOOL_ID.SCRIPT_RUN]: ItemScriptRun,
  [AI_TOOL_ID.DESKTOP_SNAPSHOT]: ItemText,
  [AI_TOOL_ID.DESKTOP_ACTION]: ItemDesktopAction,
};

export function ItemTools(props: ItemToolsProps) {
  const boundRef = useRef(false);
  const pendingRef = useRef<AiChatItem[]>([]);
  const [mcpTool, setMcpTool] = useState(false);
  const { ref, itemId, sessionId, onAskAnswer } = props;
  const childHandleRef = useRef<ItemHandle | null>(null);
  const [toolId, setToolId] = useState<number | null>(null);

  const registerChildHandle = useCallback((handle: ItemHandle | null): void => {
    if (isNil(handle)) {
      childHandleRef.current = null;
      return;
    }
    childHandleRef.current = handle;
    const pending = pendingRef.current;
    pendingRef.current = [];
    for (const eventItem of pending) {
      handle.onEvent(eventItem);
    }
  }, []);

  const dispatchToChild = useCallback((eventItem: AiChatItem): void => {
    const handle = childHandleRef.current;
    if (isNotNil(handle)) {
      handle.onEvent(eventItem);
      return;
    }
    pendingRef.current.push(eventItem);
  }, []);

  function onEvent(eventItem: AiChatItem): void {
    if (eventItem.type === RESP_EVENT_AGENT.ON_TOOL_CALL && !boundRef.current) {
      const nextToolId = resolveAiToolId(eventItem.context.toolId);
      if (isNotNil(nextToolId)) {
        boundRef.current = true;
        setToolId(nextToolId);
        setMcpTool(false);
      } else if (eventItem.context.toolId) {
        boundRef.current = true;
        setMcpTool(true);
      }
    }
    dispatchToChild(eventItem);
  }

  useItemHandle(ref, onEvent);

  function resolveView(): ReactNode {
    if (mcpTool) {
      return <ItemMcp sessionId={sessionId} ref={registerChildHandle} />;
    }
    if (isNil(toolId)) {
      return null;
    }
    const Component = TOOL_VIEW[toolId];
    if (isNil(Component)) {
      return null;
    }
    return <Component itemId={itemId} sessionId={sessionId} onAskAnswer={onAskAnswer} ref={registerChildHandle} />;
  }

  const view = resolveView();
  if (isNil(view)) {
    return null;
  }

  return <Box className="tm-agent-msg tm-agent-msg-tool">{view}</Box>;
}

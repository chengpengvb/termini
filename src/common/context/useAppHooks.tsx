import { RdpService } from '@/service/RdpService.ts';
import { SetService } from '@/service/SetService.ts';
import { SshService } from '@/service/SshService.ts';
import { SftpService } from '@/service/SftpService.ts';
import { AgentService } from '@/service/AgentService.ts';
import { GroupService } from '@/service/GroupService.ts';
import { ShellService } from '@/service/ShellService.ts';
import { AiChatService } from '@/service/AiChatService.ts';
import { AiPlanService } from '@/service/AiPlanService.ts';
import { ConfigService } from '@/service/ConfigService.ts';
import { DeviceService } from '@/service/DeviceService.ts';
import { SerialService } from '@/service/SerialService.ts';
import { TelnetService } from '@/service/TelnetService.ts';
import { useServiceContainer } from './ServiceContext.tsx';
import type { UserService } from '@/service/UserService.ts';
import { AiSkillService } from '@/service/AiSkillService.ts';
import { SnippetService } from '@/service/SnippetService.ts';
import { FileEditService } from '@/service/FileEditService.ts';
import { SftpDragService } from '@/service/SftpDragService.ts';
import { AiProjectService } from '@/service/AiProjectService.ts';
import { AiProvideService } from '@/service/AiProvideService.ts';
import { CodeIndexService } from '@/service/CodeIndexService.ts';
import { JobDeviceService } from '@/service/JobDeviceService.ts';
import { RdpServiceImpl } from '@/service/impl/RdpServiceImpl.ts';
import { SetServiceImpl } from '@/service/impl/SetServiceImpl.ts';
import { SshServiceImpl } from '@/service/impl/SshServiceImpl.ts';
import type { AccountService } from '@/service/AccountService.ts';
import { AiAgentApiService } from '@/service/AiAgentApiService.ts';
import { AiChatItemService } from '@/service/AiChatItemService.ts';
import { DeviceEditService } from '@/service/DeviceEditService.ts';
import { SystemUtilService } from '@/service/SystemUtilService.ts';
import { SftpServiceImpl } from '@/service/impl/SftpServiceImpl.ts';
import { UserServiceImpl } from '@/service/impl/UserServiceImpl.ts';
import { AiMcpServerService } from '@/service/AiMcpServerService.ts';
import { CommandRuleService } from '@/service/CommandRuleService.ts';
import { ConfirmModeService } from '@/service/ConfirmModeService.ts';
import { JobDiscoverService } from '@/service/JobDiscoverService.ts';
import { PortForwardService } from '@/service/PortForwardService.ts';
import { AgentServiceImpl } from '@/service/impl/AgentServiceImpl.ts';
import { GroupServiceImpl } from '@/service/impl/GroupServiceImpl.ts';
import { ShellServiceImpl } from '@/service/impl/ShellServiceImpl.ts';
import { AiFileChangeService } from '@/service/AiFileChangeService.ts';
import { DeviceImportService } from '@/service/DeviceImportService.ts';
import { HistoryLoginService } from '@/service/HistoryLoginService.ts';
import { JobIndicatorService } from '@/service/JobIndicatorService.ts';
import { AiChatServiceImpl } from '@/service/impl/AiChatServiceImpl.ts';
import { AiPlanServiceImpl } from '@/service/impl/AiPlanServiceImpl.ts';
import { ConfigServiceImpl } from '@/service/impl/ConfigServiceImpl.ts';
import { DeviceServiceImpl } from '@/service/impl/DeviceServiceImpl.ts';
import { SerialServiceImpl } from '@/service/impl/SerialServiceImpl.ts';
import { TelnetServiceImpl } from '@/service/impl/TelnetServiceImpl.ts';
import { ConfigKeyPairService } from '@/service/ConfigKeyPairService.ts';
import { SessionRecordService } from '@/service/SessionRecordService.ts';
import { AccountServiceImpl } from '@/service/impl/AccountServiceImpl.ts';
import { AiSkillServiceImpl } from '@/service/impl/AiSkillServiceImpl.ts';
import { SnippetServiceImpl } from '@/service/impl/SnippetServiceImpl.ts';
import { AiChat2ProjectService } from '@/service/AiChat2ProjectService.ts';
import { AiShareConsentService } from '@/service/AiShareConsentService.ts';
import { FileEditServiceImpl } from '@/service/impl/FileEditServiceImpl.ts';
import { SftpDragServiceImpl } from '@/service/impl/SftpDragServiceImpl.ts';
import { AiProjectServiceImpl } from '@/service/impl/AiProjectServiceImpl.ts';
import { AiProvideServiceImpl } from '@/service/impl/AiProvideServiceImpl.ts';
import { CodeIndexServiceImpl } from '@/service/impl/CodeIndexServiceImpl.ts';
import { JobDeviceServiceImpl } from '@/service/impl/JobDeviceServiceImpl.ts';
import type { TerminalThemeService } from '@/service/TerminalThemeService.ts';
import { AiAgentApiServiceImpl } from '@/service/impl/AiAgentApiServiceImpl.ts';
import { AiChatItemServiceImpl } from '@/service/impl/AiChatItemServiceImpl.ts';
import { DeviceEditServiceImpl } from '@/service/impl/DeviceEditServiceImpl.ts';
import { SystemUtilServiceImpl } from '@/service/impl/SystemUtilServiceImpl.ts';
import type { CommandSuggestService } from '@/service/CommandSuggestService.ts';
import { JobIndicatorAlertService } from '@/service/JobIndicatorAlertService.ts';
import { JobIndicatorValueService } from '@/service/JobIndicatorValueService.ts';
import { AiMcpServerServiceImpl } from '@/service/impl/AiMcpServerServiceImpl.ts';
import { CommandRuleServiceImpl } from '@/service/impl/CommandRuleServiceImpl.ts';
import { ConfirmModeServiceImpl } from '@/service/impl/ConfirmModeServiceImpl.ts';
import { JobDiscoverServiceImpl } from '@/service/impl/JobDiscoverServiceImpl.ts';
import { PortForwardServiceImpl } from '@/service/impl/PortForwardServiceImpl.ts';
import { AiFileChangeServiceImpl } from '@/service/impl/AiFileChangeServiceImpl.ts';
import { DeviceImportServiceImpl } from '@/service/impl/DeviceImportServiceImpl.ts';
import { HistoryLoginServiceImpl } from '@/service/impl/HistoryLoginServiceImpl.ts';
import { JobIndicatorServiceImpl } from '@/service/impl/JobIndicatorServiceImpl.ts';
import { ConfigKeyPairServiceImpl } from '@/service/impl/ConfigKeyPairServiceImpl.ts';
import { SessionRecordServiceImpl } from '@/service/impl/SessionRecordServiceImpl.ts';
import { TerminalThemeServiceImpl } from '@/service/impl/TerminalThemeServiceImpl.ts';
import type { TerminalBroadcastService } from '@/service/TerminalBroadcastService.ts';
import { AiChat2ProjectServiceImpl } from '@/service/impl/AiChat2ProjectServiceImpl.ts';
import { AiShareConsentServiceImpl } from '@/service/impl/AiShareConsentServiceImpl.ts';
import { CommandSuggestServiceImpl } from '@/service/impl/CommandSuggestServiceImpl.ts';
import { JobIndicatorAlertServiceImpl } from '@/service/impl/JobIndicatorAlertServiceImpl.ts';
import { JobIndicatorValueServiceImpl } from '@/service/impl/JobIndicatorValueServiceImpl.ts';
import { TerminalBroadcastServiceImpl } from '@/service/impl/TerminalBroadcastServiceImpl.ts';
import type { TabManagerService } from '@/common/components/tab/service/TabManagerService.ts';
import { TabManagerServiceImpl } from '@/common/components/tab/service/impl/TabManagerServiceImpl.ts';

export function useTabManagerService(): TabManagerService {
  return useServiceContainer().resolve<TabManagerService>(TabManagerServiceImpl);
}

export function useAccountService(): AccountService {
  return useServiceContainer().resolve<AccountService>(AccountServiceImpl);
}

export function useUserService(): UserService {
  return useServiceContainer().resolve<UserService>(UserServiceImpl);
}

export function useDeviceEditService(): DeviceEditService {
  return useServiceContainer().resolve<DeviceEditService>(DeviceEditServiceImpl);
}

export function useSetService(): SetService {
  return useServiceContainer().resolve<SetService>(SetServiceImpl);
}

export function useConfirmModeService(): ConfirmModeService {
  return useServiceContainer().resolve<ConfirmModeService>(ConfirmModeServiceImpl);
}

export function useAiShareConsentService(): AiShareConsentService {
  return useServiceContainer().resolve<AiShareConsentService>(AiShareConsentServiceImpl);
}

export function useConfigService(): ConfigService {
  return useServiceContainer().resolve<ConfigService>(ConfigServiceImpl);
}

export function useCodeIndexService(): CodeIndexService {
  return useServiceContainer().resolve<CodeIndexService>(CodeIndexServiceImpl);
}

export function useDeviceService(): DeviceService {
  return useServiceContainer().resolve<DeviceService>(DeviceServiceImpl);
}

export function useCommandRuleService(): CommandRuleService {
  return useServiceContainer().resolve<CommandRuleService>(CommandRuleServiceImpl);
}

export function useSnippetService(): SnippetService {
  return useServiceContainer().resolve<SnippetService>(SnippetServiceImpl);
}

export function useDeviceImportService(): DeviceImportService {
  return useServiceContainer().resolve<DeviceImportService>(DeviceImportServiceImpl);
}

export function useFileEditService(): FileEditService {
  return useServiceContainer().resolve<FileEditService>(FileEditServiceImpl);
}

export function useHistoryLoginService(): HistoryLoginService {
  return useServiceContainer().resolve<HistoryLoginService>(HistoryLoginServiceImpl);
}

export function useJobDeviceService(): JobDeviceService {
  return useServiceContainer().resolve<JobDeviceService>(JobDeviceServiceImpl);
}

export function useJobDiscoverService(): JobDiscoverService {
  return useServiceContainer().resolve<JobDiscoverService>(JobDiscoverServiceImpl);
}

export function useJobIndicatorService(): JobIndicatorService {
  return useServiceContainer().resolve<JobIndicatorService>(JobIndicatorServiceImpl);
}

export function useJobIndicatorValueService(): JobIndicatorValueService {
  return useServiceContainer().resolve<JobIndicatorValueService>(JobIndicatorValueServiceImpl);
}

export function useJobIndicatorAlertService(): JobIndicatorAlertService {
  return useServiceContainer().resolve<JobIndicatorAlertService>(JobIndicatorAlertServiceImpl);
}

export function useConfigKeyPairService(): ConfigKeyPairService {
  return useServiceContainer().resolve<ConfigKeyPairService>(ConfigKeyPairServiceImpl);
}

export function usePortForwardService(): PortForwardService {
  return useServiceContainer().resolve<PortForwardService>(PortForwardServiceImpl);
}

export function useCommandSuggestService(): CommandSuggestService {
  return useServiceContainer().resolve<CommandSuggestService>(CommandSuggestServiceImpl);
}

export function useTerminalBroadcastService(): TerminalBroadcastService {
  return useServiceContainer().resolve<TerminalBroadcastService>(TerminalBroadcastServiceImpl);
}

export function useAiChatService(): AiChatService {
  return useServiceContainer().resolve<AiChatService>(AiChatServiceImpl);
}

export function useAiProjectService(): AiProjectService {
  return useServiceContainer().resolve<AiProjectService>(AiProjectServiceImpl);
}

export function useAiChat2ProjectService(): AiChat2ProjectService {
  return useServiceContainer().resolve<AiChat2ProjectService>(AiChat2ProjectServiceImpl);
}

export function useAiChatItemService(): AiChatItemService {
  return useServiceContainer().resolve<AiChatItemService>(AiChatItemServiceImpl);
}

export function useAiFileChangeService(): AiFileChangeService {
  return useServiceContainer().resolve<AiFileChangeService>(AiFileChangeServiceImpl);
}

export function useAiProvideService(): AiProvideService {
  return useServiceContainer().resolve<AiProvideService>(AiProvideServiceImpl);
}

export function useAiSkillService(): AiSkillService {
  return useServiceContainer().resolve<AiSkillService>(AiSkillServiceImpl);
}

export function useAiMcpServerService(): AiMcpServerService {
  return useServiceContainer().resolve<AiMcpServerService>(AiMcpServerServiceImpl);
}

export function useAiAgentApiService(): AiAgentApiService {
  return useServiceContainer().resolve<AiAgentApiService>(AiAgentApiServiceImpl);
}

export function useGroupService(): GroupService {
  return useServiceContainer().resolve<GroupService>(GroupServiceImpl);
}

export function useSystemUtilService(): SystemUtilService {
  return useServiceContainer().resolve<SystemUtilService>(SystemUtilServiceImpl);
}

export function useShellService(): ShellService {
  return useServiceContainer().resolve<ShellService>(ShellServiceImpl);
}

export function useTelnetService(): TelnetService {
  return useServiceContainer().resolve<TelnetService>(TelnetServiceImpl);
}

export function useSerialService(): SerialService {
  return useServiceContainer().resolve<SerialService>(SerialServiceImpl);
}

export function useSessionRecordService(): SessionRecordService {
  return useServiceContainer().resolve<SessionRecordService>(SessionRecordServiceImpl);
}

export function useAiPlanService(): AiPlanService {
  return useServiceContainer().resolve<AiPlanService>(AiPlanServiceImpl);
}

export function useRdpService(): RdpService {
  return useServiceContainer().resolve<RdpService>(RdpServiceImpl);
}

export function useSshService(): SshService {
  return useServiceContainer().resolve<SshService>(SshServiceImpl);
}

export function useSftpService(): SftpService {
  return useServiceContainer().resolve<SftpService>(SftpServiceImpl);
}

export function useSftpDragService(): SftpDragService {
  return useServiceContainer().resolve<SftpDragService>(SftpDragServiceImpl);
}

export function useAgentService(): AgentService {
  return useServiceContainer().resolve<AgentService>(AgentServiceImpl);
}

export function useTerminalThemeService(): TerminalThemeService {
  return useServiceContainer().resolve<TerminalThemeService>(TerminalThemeServiceImpl);
}

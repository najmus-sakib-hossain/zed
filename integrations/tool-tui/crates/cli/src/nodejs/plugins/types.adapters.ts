import type { ReplyPayload } from "../../auto-reply/types.js";
import type { DxConfig } from "../../config/config.js";
import type { GroupToolPolicyConfig } from "../../config/types.tools.js";
import type { OutboundDeliveryResult, OutboundSendDeps } from "../../infra/outbound/deliver.js";
import type { RuntimeEnv } from "../../runtime.js";
import type {
  ChannelAccountSnapshot,
  ChannelAccountState,
  ChannelDirectoryEntry,
  ChannelGroupContext,
  ChannelHeartbeatDeps,
  ChannelLogSink,
  ChannelOutboundTargetMode,
  ChannelPollContext,
  ChannelPollResult,
  ChannelSecurityContext,
  ChannelSecurityDmPolicy,
  ChannelSetupInput,
  ChannelStatusIssue,
} from "./types.core.js";

export type ChannelSetupAdapter = {
  resolveAccountId?: (params: { cfg: DxConfig; accountId?: string }) => string;
  applyAccountName?: (params: {
    cfg: DxConfig;
    accountId: string;
    name?: string;
  }) => DxConfig;
  applyAccountConfig: (params: {
    cfg: DxConfig;
    accountId: string;
    input: ChannelSetupInput;
  }) => DxConfig;
  validateInput?: (params: {
    cfg: DxConfig;
    accountId: string;
    input: ChannelSetupInput;
  }) => string | null;
};

export type ChannelConfigAdapter<ResolvedAccount> = {
  listAccountIds: (cfg: DxConfig) => string[];
  resolveAccount: (cfg: DxConfig, accountId?: string | null) => ResolvedAccount;
  defaultAccountId?: (cfg: DxConfig) => string;
  setAccountEnabled?: (params: {
    cfg: DxConfig;
    accountId: string;
    enabled: boolean;
  }) => DxConfig;
  deleteAccount?: (params: { cfg: DxConfig; accountId: string }) => DxConfig;
  isEnabled?: (account: ResolvedAccount, cfg: DxConfig) => boolean;
  disabledReason?: (account: ResolvedAccount, cfg: DxConfig) => string;
  isConfigured?: (account: ResolvedAccount, cfg: DxConfig) => boolean | Promise<boolean>;
  unconfiguredReason?: (account: ResolvedAccount, cfg: DxConfig) => string;
  describeAccount?: (account: ResolvedAccount, cfg: DxConfig) => ChannelAccountSnapshot;
  resolveAllowFrom?: (params: {
    cfg: DxConfig;
    accountId?: string | null;
  }) => string[] | undefined;
  formatAllowFrom?: (params: {
    cfg: DxConfig;
    accountId?: string | null;
    allowFrom: Array<string | number>;
  }) => string[];
};

export type ChannelGroupAdapter = {
  resolveRequireMention?: (params: ChannelGroupContext) => boolean | undefined;
  resolveGroupIntroHint?: (params: ChannelGroupContext) => string | undefined;
  resolveToolPolicy?: (params: ChannelGroupContext) => GroupToolPolicyConfig | undefined;
};

export type ChannelOutboundContext = {
  cfg: DxConfig;
  to: string;
  text: string;
  mediaUrl?: string;
  gifPlayback?: boolean;
  replyToId?: string | null;
  threadId?: string | number | null;
  accountId?: string | null;
  deps?: OutboundSendDeps;
};

export type ChannelOutboundPayloadContext = ChannelOutboundContext & {
  payload: ReplyPayload;
};

export type ChannelOutboundAdapter = {
  deliveryMode: "direct" | "gateway" | "hybrid";
  chunker?: ((text: string, limit: number) => string[]) | null;
  chunkerMode?: "text" | "markdown";
  textChunkLimit?: number;
  pollMaxOptions?: number;
  resolveTarget?: (params: {
    cfg?: DxConfig;
    to?: string;
    allowFrom?: string[];
    accountId?: string | null;
    mode?: ChannelOutboundTargetMode;
  }) => { ok: true; to: string } | { ok: false; error: Error };
  sendPayload?: (ctx: ChannelOutboundPayloadContext) => Promise<OutboundDeliveryResult>;
  sendText?: (ctx: ChannelOutboundContext) => Promise<OutboundDeliveryResult>;
  sendMedia?: (ctx: ChannelOutboundContext) => Promise<OutboundDeliveryResult>;
  sendPoll?: (ctx: ChannelPollContext) => Promise<ChannelPollResult>;
};

export type ChannelStatusAdapter<ResolvedAccount, Probe = unknown, Audit = unknown> = {
  defaultRuntime?: ChannelAccountSnapshot;
  buildChannelSummary?: (params: {
    account: ResolvedAccount;
    cfg: DxConfig;
    defaultAccountId: string;
    snapshot: ChannelAccountSnapshot;
  }) => Record<string, unknown> | Promise<Record<string, unknown>>;
  probeAccount?: (params: {
    account: ResolvedAccount;
    timeoutMs: number;
    cfg: DxConfig;
  }) => Promise<Probe>;
  auditAccount?: (params: {
    account: ResolvedAccount;
    timeoutMs: number;
    cfg: DxConfig;
    probe?: Probe;
  }) => Promise<Audit>;
  buildAccountSnapshot?: (params: {
    account: ResolvedAccount;
    cfg: DxConfig;
    runtime?: ChannelAccountSnapshot;
    probe?: Probe;
    audit?: Audit;
  }) => ChannelAccountSnapshot | Promise<ChannelAccountSnapshot>;
  logSelfId?: (params: {
    account: ResolvedAccount;
    cfg: DxConfig;
    runtime: RuntimeEnv;
    includeChannelPrefix?: boolean;
  }) => void;
  resolveAccountState?: (params: {
    account: ResolvedAccount;
    cfg: DxConfig;
    configured: boolean;
    enabled: boolean;
  }) => ChannelAccountState;
  collectStatusIssues?: (accounts: ChannelAccountSnapshot[]) => ChannelStatusIssue[];
};

export type ChannelGatewayContext<ResolvedAccount = unknown> = {
  cfg: DxConfig;
  accountId: string;
  account: ResolvedAccount;
  runtime: RuntimeEnv;
  abortSignal: AbortSignal;
  log?: ChannelLogSink;
  getStatus: () => ChannelAccountSnapshot;
  setStatus: (next: ChannelAccountSnapshot) => void;
};

export type ChannelLogoutResult = {
  cleared: boolean;
  loggedOut?: boolean;
  [key: string]: unknown;
};

export type ChannelLoginWithQrStartResult = {
  qrDataUrl?: string;
  message: string;
};

export type ChannelLoginWithQrWaitResult = {
  connected: boolean;
  message: string;
};

export type ChannelLogoutContext<ResolvedAccount = unknown> = {
  cfg: DxConfig;
  accountId: string;
  account: ResolvedAccount;
  runtime: RuntimeEnv;
  log?: ChannelLogSink;
};

export type ChannelPairingAdapter = {
  idLabel: string;
  normalizeAllowEntry?: (entry: string) => string;
  notifyApproval?: (params: {
    cfg: DxConfig;
    id: string;
    runtime?: RuntimeEnv;
  }) => Promise<void>;
};

export type ChannelGatewayAdapter<ResolvedAccount = unknown> = {
  startAccount?: (ctx: ChannelGatewayContext<ResolvedAccount>) => Promise<unknown>;
  stopAccount?: (ctx: ChannelGatewayContext<ResolvedAccount>) => Promise<void>;
  loginWithQrStart?: (params: {
    accountId?: string;
    force?: boolean;
    timeoutMs?: number;
    verbose?: boolean;
  }) => Promise<ChannelLoginWithQrStartResult>;
  loginWithQrWait?: (params: {
    accountId?: string;
    timeoutMs?: number;
  }) => Promise<ChannelLoginWithQrWaitResult>;
  logoutAccount?: (ctx: ChannelLogoutContext<ResolvedAccount>) => Promise<ChannelLogoutResult>;
};

export type ChannelAuthAdapter = {
  login?: (params: {
    cfg: DxConfig;
    accountId?: string | null;
    runtime: RuntimeEnv;
    verbose?: boolean;
    channelInput?: string | null;
  }) => Promise<void>;
};

export type ChannelHeartbeatAdapter = {
  checkReady?: (params: {
    cfg: DxConfig;
    accountId?: string | null;
    deps?: ChannelHeartbeatDeps;
  }) => Promise<{ ok: boolean; reason: string }>;
  resolveRecipients?: (params: { cfg: DxConfig; opts?: { to?: string; all?: boolean } }) => {
    recipients: string[];
    source: string;
  };
};

export type ChannelDirectoryAdapter = {
  self?: (params: {
    cfg: DxConfig;
    accountId?: string | null;
    runtime: RuntimeEnv;
  }) => Promise<ChannelDirectoryEntry | null>;
  listPeers?: (params: {
    cfg: DxConfig;
    accountId?: string | null;
    query?: string | null;
    limit?: number | null;
    runtime: RuntimeEnv;
  }) => Promise<ChannelDirectoryEntry[]>;
  listPeersLive?: (params: {
    cfg: DxConfig;
    accountId?: string | null;
    query?: string | null;
    limit?: number | null;
    runtime: RuntimeEnv;
  }) => Promise<ChannelDirectoryEntry[]>;
  listGroups?: (params: {
    cfg: DxConfig;
    accountId?: string | null;
    query?: string | null;
    limit?: number | null;
    runtime: RuntimeEnv;
  }) => Promise<ChannelDirectoryEntry[]>;
  listGroupsLive?: (params: {
    cfg: DxConfig;
    accountId?: string | null;
    query?: string | null;
    limit?: number | null;
    runtime: RuntimeEnv;
  }) => Promise<ChannelDirectoryEntry[]>;
  listGroupMembers?: (params: {
    cfg: DxConfig;
    accountId?: string | null;
    groupId: string;
    limit?: number | null;
    runtime: RuntimeEnv;
  }) => Promise<ChannelDirectoryEntry[]>;
};

export type ChannelResolveKind = "user" | "group";

export type ChannelResolveResult = {
  input: string;
  resolved: boolean;
  id?: string;
  name?: string;
  note?: string;
};

export type ChannelResolverAdapter = {
  resolveTargets: (params: {
    cfg: DxConfig;
    accountId?: string | null;
    inputs: string[];
    kind: ChannelResolveKind;
    runtime: RuntimeEnv;
  }) => Promise<ChannelResolveResult[]>;
};

export type ChannelElevatedAdapter = {
  allowFromFallback?: (params: {
    cfg: DxConfig;
    accountId?: string | null;
  }) => Array<string | number> | undefined;
};

export type ChannelCommandAdapter = {
  enforceOwnerForCommands?: boolean;
  skipWhenConfigEmpty?: boolean;
};

export type ChannelSecurityAdapter<ResolvedAccount = unknown> = {
  resolveDmPolicy?: (
    ctx: ChannelSecurityContext<ResolvedAccount>,
  ) => ChannelSecurityDmPolicy | null;
  collectWarnings?: (ctx: ChannelSecurityContext<ResolvedAccount>) => Promise<string[]> | string[];
};

import { useCallback, useEffect, useMemo, useState } from "react";
import { motion } from "framer-motion";
import {
  AlertCircle,
  ArrowUpCircle,
  CheckCircle2,
  Database,
  Download,
  Loader2,
  RefreshCw,
  Server,
  Stethoscope,
} from "lucide-react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import {
  Accordion,
  AccordionContent,
  AccordionItem,
  AccordionTrigger,
} from "@/components/ui/accordion";
import { Badge } from "@/components/ui/badge";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { ImportExportSection } from "@/components/settings/ImportExportSection";
import { useImportExport } from "@/hooks/useImportExport";
import { remoteApi } from "@/lib/api";
import type {
  ManagementTarget,
  RemoteHealth,
  RemoteToolVersion,
} from "@/lib/api";
import { isUpdateAvailable } from "@/lib/version";
import { extractErrorMessage } from "@/utils/errorUtils";

interface RemoteSettingsPageProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onImportSuccess?: () => void | Promise<void>;
  defaultTab?: string;
  target: Extract<ManagementTarget, { type: "remote" }>;
}

const TOOL_NAMES = [
  "claude",
  "codex",
  "gemini",
  "opencode",
  "openclaw",
  "hermes",
] as const;

type ToolName = (typeof TOOL_NAMES)[number];
type ToolLifecycleAction = "install" | "update";

const TOOL_DISPLAY_NAMES: Record<ToolName, string> = {
  claude: "Claude Code",
  codex: "Codex CLI",
  gemini: "Gemini CLI",
  opencode: "OpenCode",
  openclaw: "OpenClaw",
  hermes: "Hermes",
};

function coerceRemoteTab(tab: string | undefined): string {
  if (tab === "advanced") return "data";
  if (tab === "about") return "environment";
  return tab === "data" || tab === "environment" ? tab : "environment";
}

export function RemoteSettingsPage({
  open,
  onImportSuccess,
  defaultTab = "environment",
  target,
}: RemoteSettingsPageProps) {
  const { t } = useTranslation();
  const [activeTab, setActiveTab] = useState(coerceRemoteTab(defaultTab));

  const [health, setHealth] = useState<RemoteHealth | null>(null);
  const [isCheckingHealth, setIsCheckingHealth] = useState(false);
  const [isInstallingHelper, setIsInstallingHelper] = useState(false);
  const [toolVersions, setToolVersions] = useState<RemoteToolVersion[]>([]);
  const [isLoadingTools, setIsLoadingTools] = useState(false);
  const [toolActions, setToolActions] = useState<
    Partial<Record<ToolName, ToolLifecycleAction>>
  >({});
  const [batchAction, setBatchAction] = useState<ToolLifecycleAction | null>(
    null,
  );

  const importExport = useImportExport({ onImportSuccess, target });

  const toolsCapability = health?.capabilities.includes("tools") ?? false;

  const toolVersionByName = useMemo(
    () => new Map(toolVersions.map((tool) => [tool.name, tool])),
    [toolVersions],
  );

  const updatableTools = useMemo(
    () =>
      TOOL_NAMES.filter((name) => {
        const tool = toolVersionByName.get(name);
        return Boolean(
          tool?.version &&
            tool.latest_version &&
            isUpdateAvailable(tool.version, tool.latest_version),
        );
      }),
    [toolVersionByName],
  );

  const loadHealth = useCallback(async () => {
    setIsCheckingHealth(true);
    try {
      const result = await remoteApi.checkHealth(target.profile, target.secret);
      setHealth(result);
      return result;
    } catch (error) {
      console.error("[RemoteSettingsPage] Failed to check health", error);
      setHealth({
        reachable: false,
        helperInstalled: false,
        capabilities: [],
        lastError: extractErrorMessage(error),
      });
      return null;
    } finally {
      setIsCheckingHealth(false);
    }
  }, [target.profile, target.secret]);

  const loadToolVersions = useCallback(async () => {
    setIsLoadingTools(true);
    try {
      const result = await remoteApi.getToolVersions(
        target.profile,
        [...TOOL_NAMES],
        target.secret,
      );
      setToolVersions(result);
      return result;
    } catch (error) {
      console.error("[RemoteSettingsPage] Failed to load tools", error);
      toast.error(
        t("remote.settings.environment.loadToolsFailed", {
          defaultValue: "远程环境检查失败",
        }),
        { description: extractErrorMessage(error) },
      );
      return [];
    } finally {
      setIsLoadingTools(false);
    }
  }, [t, target.profile, target.secret]);

  useEffect(() => {
    if (!open) return;
    setActiveTab(coerceRemoteTab(defaultTab));
    void loadHealth();
  }, [defaultTab, loadHealth, open]);

  const installHelper = async () => {
    setIsInstallingHelper(true);
    try {
      const result = await remoteApi.installHelper(
        target.profile,
        target.secret,
      );
      setHealth(result);
      toast.success(
        t("remote.health.installSuccess", {
          defaultValue: "远程 Helper 已安装",
        }),
      );
    } catch (error) {
      console.error("[RemoteSettingsPage] Failed to install helper", error);
      toast.error(
        t("remote.health.installFailed", {
          defaultValue: "安装远程 Helper 失败: {{error}}",
          error: extractErrorMessage(error),
        }),
      );
    } finally {
      setIsInstallingHelper(false);
    }
  };

  const runToolAction = async (
    toolNames: ToolName[],
    action: ToolLifecycleAction,
  ) => {
    if (toolNames.length === 0) return;
    const isBatch = toolNames.length > 1;
    if (isBatch) setBatchAction(action);

    const failures: string[] = [];
    let succeeded = 0;
    for (const toolName of toolNames) {
      setToolActions((prev) => ({ ...prev, [toolName]: action }));
      try {
        await remoteApi.runToolLifecycleAction(
          target.profile,
          [toolName],
          action,
          target.secret,
        );
        succeeded += 1;
        await loadToolVersions();
      } catch (error) {
        console.error(
          `[RemoteSettingsPage] Failed to ${action} ${toolName}`,
          error,
        );
        failures.push(
          `${TOOL_DISPLAY_NAMES[toolName]}: ${extractErrorMessage(error)}`,
        );
      } finally {
        setToolActions((prev) => {
          const next = { ...prev };
          delete next[toolName];
          return next;
        });
      }
    }

    if (isBatch) setBatchAction(null);
    const actionLabel =
      action === "install"
        ? t("settings.toolInstall")
        : t("settings.toolUpdate");
    if (failures.length === 0) {
      toast.success(
        t("settings.toolActionDone", {
          count: succeeded,
          action: actionLabel,
        }),
      );
    } else if (succeeded === 0) {
      toast.error(t("settings.toolActionFailed"), {
        description: failures.join("\n"),
      });
    } else {
      toast.warning(
        t("settings.toolActionPartial", {
          succeeded,
          failed: failures.length,
        }),
        { description: failures.join("\n") },
      );
    }
  };

  return (
    <div className="flex flex-col h-full overflow-hidden px-6">
      <span data-testid="settings-target" className="sr-only">
        remote
      </span>
      <Tabs
        value={activeTab}
        onValueChange={setActiveTab}
        className="flex flex-col h-full"
      >
        <TabsList className="grid w-full grid-cols-2 mb-6 glass rounded-lg">
          <TabsTrigger value="environment">
            {t("remote.settings.tabs.environment", {
              defaultValue: "远程环境",
            })}
          </TabsTrigger>
          <TabsTrigger value="data">
            {t("remote.settings.tabs.data", { defaultValue: "数据" })}
          </TabsTrigger>
        </TabsList>

        <div className="flex-1 min-h-0 overflow-y-auto overflow-x-hidden pr-2">
          <TabsContent value="environment" className="space-y-4 mt-0 pb-4">
            <motion.div
              initial={{ opacity: 0, y: 10 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ duration: 0.3 }}
              className="space-y-4"
            >
              <RemoteHealthSection
                health={health}
                isChecking={isCheckingHealth}
                isInstalling={isInstallingHelper}
                onRefresh={() => {
                  void loadHealth();
                }}
                onInstallHelper={installHelper}
                profileName={target.profile.name || target.profile.host}
              />
              <RemoteToolEnvironmentSection
                disabled={!toolsCapability}
                toolVersions={toolVersions}
                isLoading={isLoadingTools}
                toolActions={toolActions}
                batchAction={batchAction}
                updatableTools={updatableTools}
                onRefresh={() => {
                  void loadToolVersions();
                }}
                onRunAction={runToolAction}
              />
            </motion.div>
          </TabsContent>

          <TabsContent value="data" className="space-y-4 mt-0 pb-4">
            <motion.div
              initial={{ opacity: 0, y: 10 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ duration: 0.3 }}
              className="space-y-4"
            >
              <Accordion
                type="multiple"
                defaultValue={["data"]}
                className="space-y-4"
              >
                <AccordionItem
                  value="data"
                  className="rounded-xl glass-card overflow-hidden"
                >
                  <AccordionTrigger className="px-6 py-4 hover:no-underline hover:bg-muted/50 data-[state=open]:bg-muted/50">
                    <div className="flex items-center gap-3">
                      <Database className="h-5 w-5 text-blue-500" />
                      <div className="text-left">
                        <h3 className="text-base font-semibold">
                          {t("settings.advanced.data.title")}
                        </h3>
                        <p className="text-sm text-muted-foreground font-normal">
                          {t("remote.settings.data.description", {
                            defaultValue:
                              "导入或导出当前远程服务器自己的 CC Switch 数据",
                          })}
                        </p>
                      </div>
                    </div>
                  </AccordionTrigger>
                  <AccordionContent className="px-6 pb-6 pt-4 border-t border-border/50">
                    <ImportExportSection
                      status={importExport.status}
                      selectedFile={importExport.selectedFile}
                      errorMessage={importExport.errorMessage}
                      backupId={importExport.backupId}
                      isImporting={importExport.isImporting}
                      onSelectFile={importExport.selectImportFile}
                      onImport={importExport.importConfig}
                      onExport={importExport.exportConfig}
                      onClear={importExport.clearSelection}
                    />
                  </AccordionContent>
                </AccordionItem>
              </Accordion>
            </motion.div>
          </TabsContent>
        </div>
      </Tabs>
    </div>
  );
}

interface RemoteHealthSectionProps {
  health: RemoteHealth | null;
  isChecking: boolean;
  isInstalling: boolean;
  profileName: string;
  onRefresh: () => void | Promise<void>;
  onInstallHelper: () => void | Promise<void>;
}

function RemoteHealthSection({
  health,
  isChecking,
  isInstalling,
  profileName,
  onRefresh,
  onInstallHelper,
}: RemoteHealthSectionProps) {
  const { t } = useTranslation();
  const helperReady = Boolean(health?.reachable && health.helperInstalled);
  return (
    <div className="rounded-xl glass-card overflow-hidden">
      <div className="px-6 py-4 border-b border-border/50 flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
        <div className="flex min-w-0 items-center gap-3">
          <Server className="h-5 w-5 text-primary shrink-0" />
          <div className="min-w-0">
            <h3 className="text-base font-semibold truncate">
              {t("remote.settings.environment.host", {
                defaultValue: "远程主机",
              })}
            </h3>
            <p className="text-sm text-muted-foreground truncate">
              {profileName}
            </p>
          </div>
          <Badge variant={helperReady ? "default" : "destructive"}>
            {helperReady
              ? t("remote.status.connected", { defaultValue: "已连接" })
              : t("remote.status.unavailable", { defaultValue: "不可用" })}
          </Badge>
        </div>
        <div className="flex shrink-0 gap-2">
          <Button
            type="button"
            variant="outline"
            size="sm"
            onClick={() => void onRefresh()}
            disabled={isChecking}
          >
            {isChecking ? (
              <Loader2 className="mr-2 h-4 w-4 animate-spin" />
            ) : (
              <RefreshCw className="mr-2 h-4 w-4" />
            )}
            {t("common.refresh")}
          </Button>
          <Button
            type="button"
            variant="outline"
            size="sm"
            onClick={() => void onInstallHelper()}
            disabled={isInstalling}
          >
            {isInstalling ? (
              <Loader2 className="mr-2 h-4 w-4 animate-spin" />
            ) : (
              <Download className="mr-2 h-4 w-4" />
            )}
            {t("remote.health.installHelper", {
              defaultValue: "安装 Helper",
            })}
          </Button>
        </div>
      </div>
      <div className="grid gap-px bg-border/50 sm:grid-cols-3">
        <InfoCell
          label={t("remote.health.helperVersion", {
            defaultValue: "Helper 版本",
          })}
          value={health?.helperVersion ?? "-"}
        />
        <InfoCell
          label={t("remote.health.platform", { defaultValue: "系统" })}
          value={health?.platform ?? "-"}
        />
        <InfoCell
          label={t("remote.health.capabilities", { defaultValue: "能力" })}
          value={health?.capabilities.join(", ") || "-"}
        />
      </div>
      {health?.lastError ? (
        <div className="px-6 py-3 border-t border-border/50 text-sm text-destructive">
          {health.lastError}
        </div>
      ) : null}
    </div>
  );
}

function InfoCell({ label, value }: { label: string; value: string }) {
  return (
    <div className="bg-background/70 px-4 py-3 min-w-0">
      <div className="text-xs text-muted-foreground">{label}</div>
      <div className="mt-1 truncate text-sm font-medium">{value}</div>
    </div>
  );
}

interface RemoteToolEnvironmentSectionProps {
  disabled: boolean;
  toolVersions: RemoteToolVersion[];
  isLoading: boolean;
  toolActions: Partial<Record<ToolName, ToolLifecycleAction>>;
  batchAction: ToolLifecycleAction | null;
  updatableTools: readonly ToolName[];
  onRefresh: () => void | Promise<void>;
  onRunAction: (
    tools: ToolName[],
    action: ToolLifecycleAction,
  ) => void | Promise<void>;
}

function RemoteToolEnvironmentSection({
  disabled,
  toolVersions,
  isLoading,
  toolActions,
  batchAction,
  updatableTools,
  onRefresh,
  onRunAction,
}: RemoteToolEnvironmentSectionProps) {
  const { t } = useTranslation();
  const toolVersionByName = useMemo(
    () => new Map(toolVersions.map((tool) => [tool.name, tool])),
    [toolVersions],
  );

  return (
    <div className="rounded-xl glass-card overflow-hidden">
      <div className="px-6 py-4 border-b border-border/50 flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
        <div className="flex min-w-0 items-center gap-3">
          <Stethoscope className="h-5 w-5 text-primary shrink-0" />
          <div className="min-w-0">
            <h3 className="text-base font-semibold">
              {t("remote.settings.environment.toolsTitle", {
                defaultValue: "远程环境检查更新",
              })}
            </h3>
            <p className="text-sm text-muted-foreground">
              {t("remote.settings.environment.toolsDescription", {
                defaultValue:
                  "检查远程服务器上的 Claude Code、Codex、Gemini、OpenCode、OpenClaw 和 Hermes",
              })}
            </p>
          </div>
        </div>
        <div className="flex shrink-0 flex-wrap gap-2">
          <Button
            type="button"
            variant="outline"
            size="sm"
            onClick={() => void onRefresh()}
            disabled={disabled || isLoading}
          >
            {isLoading ? (
              <Loader2 className="mr-2 h-4 w-4 animate-spin" />
            ) : (
              <RefreshCw className="mr-2 h-4 w-4" />
            )}
            {t("common.refresh")}
          </Button>
          <Button
            type="button"
            size="sm"
            onClick={() => void onRunAction([...updatableTools], "update")}
            disabled={disabled || updatableTools.length === 0 || !!batchAction}
          >
            {batchAction === "update" ? (
              <Loader2 className="mr-2 h-4 w-4 animate-spin" />
            ) : (
              <ArrowUpCircle className="mr-2 h-4 w-4" />
            )}
            {t("settings.updateAllTools", { count: updatableTools.length })}
          </Button>
        </div>
      </div>

      {disabled ? (
        <div className="px-6 py-4 text-sm text-muted-foreground">
          {t("remote.settings.environment.helperUnsupported", {
            defaultValue:
              "当前远程 Helper 不支持环境检查更新。请安装包含 tools capability 的新版 Helper。",
          })}
        </div>
      ) : (
        <div className="grid gap-px bg-border/50 md:grid-cols-2 xl:grid-cols-3">
          {TOOL_NAMES.map((name) => {
            const tool = toolVersionByName.get(name);
            const action = toolActions[name];
            const hasUpdate = Boolean(
              tool?.version &&
                tool.latest_version &&
                isUpdateAvailable(tool.version, tool.latest_version),
            );
            const installed = Boolean(tool?.version);
            return (
              <div key={name} className="bg-background/70 p-4 min-w-0">
                <div className="flex items-start justify-between gap-3">
                  <div className="min-w-0">
                    <div className="font-medium truncate">
                      {TOOL_DISPLAY_NAMES[name]}
                    </div>
                    <div className="mt-1 text-xs text-muted-foreground truncate">
                      {tool?.env_type ?? "-"}
                    </div>
                  </div>
                  <ToolStatusBadge tool={tool} hasUpdate={hasUpdate} />
                </div>
                <div className="mt-4 grid grid-cols-2 gap-3 text-sm">
                  <InfoLine
                    label={t("settings.currentVersion")}
                    value={tool?.version ?? "-"}
                  />
                  <InfoLine
                    label={t("settings.latestVersion")}
                    value={tool?.latest_version ?? "-"}
                  />
                </div>
                {tool?.error ? (
                  <div className="mt-3 line-clamp-2 text-xs text-destructive">
                    {tool.error}
                  </div>
                ) : null}
                <div className="mt-4 flex justify-end gap-2">
                  <Button
                    type="button"
                    size="sm"
                    variant="outline"
                    onClick={() =>
                      void onRunAction([name], installed ? "update" : "install")
                    }
                    disabled={isLoading || !!action || !!batchAction}
                  >
                    {action ? (
                      <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                    ) : installed ? (
                      <ArrowUpCircle className="mr-2 h-4 w-4" />
                    ) : (
                      <Download className="mr-2 h-4 w-4" />
                    )}
                    {installed
                      ? t("settings.toolUpdate")
                      : t("settings.toolInstall")}
                  </Button>
                </div>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}

function ToolStatusBadge({
  tool,
  hasUpdate,
}: {
  tool?: RemoteToolVersion;
  hasUpdate: boolean;
}) {
  const { t } = useTranslation();
  if (hasUpdate) {
    return (
      <Badge variant="secondary">
        <ArrowUpCircle className="mr-1 h-3 w-3" />
        {t("settings.updateAvailableShort", {
          defaultValue: "可升级",
        })}
      </Badge>
    );
  }
  if (tool?.version) {
    return (
      <Badge variant="default">
        <CheckCircle2 className="mr-1 h-3 w-3" />
        {t("settings.toolReady")}
      </Badge>
    );
  }
  return (
    <Badge variant="destructive">
      <AlertCircle className="mr-1 h-3 w-3" />
      {t("settings.notInstalled", { defaultValue: "未安装" })}
    </Badge>
  );
}

function InfoLine({ label, value }: { label: string; value: string }) {
  return (
    <div className="min-w-0">
      <div className="text-xs text-muted-foreground">{label}</div>
      <div className="mt-1 truncate">{value}</div>
    </div>
  );
}

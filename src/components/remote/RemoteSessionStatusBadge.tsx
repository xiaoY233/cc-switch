import { Loader2, Plug, PlugZap, Unplug } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Badge } from "@/components/ui/badge";
import type { RemoteSessionStatus } from "@/lib/api";
import { cn } from "@/lib/utils";

interface RemoteSessionStatusBadgeProps {
  status?: RemoteSessionStatus | null;
  compact?: boolean;
  className?: string;
}

export function RemoteSessionStatusBadge({
  status,
  compact = false,
  className,
}: RemoteSessionStatusBadgeProps) {
  const { t } = useTranslation();
  const state = status?.state ?? "idle";
  const label = getRemoteSessionStatusLabel(state, t);
  const busy =
    state === "connecting" || state === "busy" || state === "reconnecting";
  const failed = state === "failed";
  const ready = state === "ready";
  const Icon = busy ? Loader2 : ready ? PlugZap : failed ? Unplug : Plug;

  return (
    <Badge
      variant="outline"
      className={cn(
        "h-6 max-w-full gap-1.5 whitespace-nowrap border-border-default bg-background/70 px-2 text-[11px] font-medium text-muted-foreground shadow-none",
        busy &&
          "border-blue-500/25 bg-blue-500/[0.06] text-blue-600 dark:text-blue-400",
        ready &&
          "border-emerald-500/25 bg-emerald-500/[0.06] text-emerald-600 dark:text-emerald-400",
        failed &&
          "border-red-500/30 bg-background text-red-500 dark:text-red-400",
        className,
      )}
      title={status?.lastError || label}
      data-testid="remote-session-status"
    >
      <Icon className={cn("h-3 w-3 shrink-0", busy && "animate-spin")} />
      {!compact && <span className="truncate">{label}</span>}
    </Badge>
  );
}

function getRemoteSessionStatusLabel(
  state: RemoteSessionStatus["state"] | "idle",
  t: (key: string, options?: any) => string,
) {
  switch (state) {
    case "connecting":
      return t("remote.sessionStatus.connecting", { defaultValue: "连接中" });
    case "ready":
      return t("remote.sessionStatus.ready", { defaultValue: "已连接" });
    case "busy":
      return t("remote.sessionStatus.busy", { defaultValue: "执行中" });
    case "reconnecting":
      return t("remote.sessionStatus.reconnecting", {
        defaultValue: "重连中",
      });
    case "failed":
      return t("remote.sessionStatus.failed", { defaultValue: "连接失败" });
    case "closed":
      return t("remote.sessionStatus.closed", { defaultValue: "已断开" });
    case "idle":
    default:
      return t("remote.sessionStatus.idle", { defaultValue: "未连接" });
  }
}

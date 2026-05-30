import type { CSSProperties } from "react";
import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import { Check, ChevronDown, LayoutDashboard, Server } from "lucide-react";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { cn } from "@/lib/utils";
import type { RemoteHostProfile } from "@/lib/api";

interface ManagementTargetSwitcherProps {
  profiles: RemoteHostProfile[];
  activeTargetKey: string;
  onTargetChange: (targetKey: string) => void;
  onManageServers?: () => void;
  className?: string;
  style?: CSSProperties;
}

export function ManagementTargetSwitcher({
  profiles,
  activeTargetKey,
  onTargetChange,
  onManageServers,
  className,
  style,
}: ManagementTargetSwitcherProps) {
  const { t } = useTranslation();
  const activeRemoteProfile = useMemo(
    () =>
      profiles.find((profile) => `remote:${profile.id}` === activeTargetKey),
    [activeTargetKey, profiles],
  );
  const isLocal = activeTargetKey === "local" || !activeRemoteProfile;

  return (
    <div
      className={cn("inline-flex rounded-xl bg-muted p-1", className)}
      style={style}
      aria-label={t("remote.targetSelector", { defaultValue: "管理目标" })}
    >
      <button
        type="button"
        onClick={() => onTargetChange("local")}
        className={targetButtonClass(isLocal)}
      >
        <LayoutDashboard className="h-4 w-4" />
        <span>{t("remote.localTarget", { defaultValue: "本地" })}</span>
      </button>

      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <button
            type="button"
            className={cn(targetButtonClass(!isLocal), "max-w-[180px]")}
          >
            <Server className="h-4 w-4 shrink-0" />
            <span className="truncate">
              {activeRemoteProfile?.name ??
                t("remote.remoteTarget", { defaultValue: "远程" })}
            </span>
            <ChevronDown className="h-3.5 w-3.5 shrink-0 opacity-60" />
          </button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="start" className="w-64">
          <DropdownMenuLabel>
            {t("remote.manageServers", { defaultValue: "远程服务器" })}
          </DropdownMenuLabel>
          {profiles.length === 0 ? (
            <DropdownMenuItem disabled>
              {t("remote.noServers", { defaultValue: "暂无远程服务器" })}
            </DropdownMenuItem>
          ) : (
            profiles.map((profile) => {
              const targetKey = `remote:${profile.id}`;
              const selected = targetKey === activeTargetKey;
              return (
                <DropdownMenuItem
                  key={profile.id}
                  onSelect={() => onTargetChange(targetKey)}
                  className="min-w-0"
                >
                  <Server className="h-4 w-4 shrink-0 text-muted-foreground" />
                  <span className="min-w-0 flex-1 truncate">
                    {profile.name}
                  </span>
                  {selected && <Check className="h-4 w-4 text-primary" />}
                </DropdownMenuItem>
              );
            })
          )}
          {onManageServers && (
            <>
              <DropdownMenuSeparator />
              <DropdownMenuItem onSelect={onManageServers}>
                <Server className="h-4 w-4 text-muted-foreground" />
                {t("remote.manageServers", { defaultValue: "远程服务器" })}
              </DropdownMenuItem>
            </>
          )}
        </DropdownMenuContent>
      </DropdownMenu>
    </div>
  );
}

function targetButtonClass(active: boolean) {
  return cn(
    "inline-flex h-8 items-center gap-2 rounded-md px-3 text-sm font-medium transition-all duration-200",
    active
      ? "bg-background text-foreground shadow-sm"
      : "text-muted-foreground hover:bg-background/50 hover:text-foreground",
  );
}

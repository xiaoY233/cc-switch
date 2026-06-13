import { useTranslation } from "react-i18next";
import type { Settings } from "@/types";
import { AppWindow, MonitorUp, Power, EyeOff } from "lucide-react";
import { ToggleRow } from "@/components/ui/toggle-row";
import { AnimatePresence, motion } from "framer-motion";
import { isLinux } from "@/lib/platform";

type WindowSettingsState = Pick<
  Settings,
  | "launchOnStartup"
  | "silentStartup"
  | "enableClaudePluginIntegration"
  | "skipClaudeOnboarding"
  | "minimizeToTrayOnClose"
  | "useAppWindowControls"
>;

interface WindowSettingsProps {
  settings: WindowSettingsState;
  onChange: (updates: Partial<WindowSettingsState>) => void;
  mode?: "local" | "remote";
  disabled?: boolean;
}

export function WindowSettings({
  settings,
  onChange,
  mode = "local",
  disabled = false,
}: WindowSettingsProps) {
  const { t } = useTranslation();
  const isRemote = mode === "remote";

  return (
    <section className="space-y-4">
      <div className="flex items-center gap-2 pb-2 border-b border-border/40">
        <AppWindow className="h-4 w-4 text-primary" />
        <h3 className="text-sm font-medium">{t("settings.windowBehavior")}</h3>
      </div>

      <div className="space-y-3">
        {!isRemote ? (
          <ToggleRow
            icon={<Power className="h-4 w-4 text-orange-500" />}
            title={t("settings.launchOnStartup")}
            description={t("settings.launchOnStartupDescription")}
            checked={!!settings.launchOnStartup}
            onCheckedChange={(value) => onChange({ launchOnStartup: value })}
          />
        ) : null}

        <AnimatePresence initial={false}>
          {!isRemote && settings.launchOnStartup && (
            <motion.div
              key="silent-startup"
              initial={{ opacity: 0, y: 10 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: 10 }}
              transition={{ duration: 0.3 }}
            >
              <ToggleRow
                icon={<EyeOff className="h-4 w-4 text-green-500" />}
                title={t("settings.silentStartup")}
                description={t("settings.silentStartupDescription")}
                checked={!!settings.silentStartup}
                onCheckedChange={(value) => onChange({ silentStartup: value })}
                disabled={disabled}
              />
            </motion.div>
          )}
        </AnimatePresence>

        <ToggleRow
          icon={<MonitorUp className="h-4 w-4 text-purple-500" />}
          title={t("settings.enableClaudePluginIntegration")}
          description={t("settings.enableClaudePluginIntegrationDescription")}
          checked={!!settings.enableClaudePluginIntegration}
          onCheckedChange={(value) =>
            onChange({ enableClaudePluginIntegration: value })
          }
          disabled={disabled}
        />

        <ToggleRow
          icon={<MonitorUp className="h-4 w-4 text-cyan-500" />}
          title={t("settings.skipClaudeOnboarding")}
          description={t("settings.skipClaudeOnboardingDescription")}
          checked={!!settings.skipClaudeOnboarding}
          onCheckedChange={(value) => onChange({ skipClaudeOnboarding: value })}
          disabled={disabled}
        />

        {!isRemote ? (
          <ToggleRow
            icon={<AppWindow className="h-4 w-4 text-blue-500" />}
            title={t("settings.minimizeToTray")}
            description={t("settings.minimizeToTrayDescription")}
            checked={!!settings.minimizeToTrayOnClose}
            onCheckedChange={(value) =>
              onChange({ minimizeToTrayOnClose: value })
            }
          />
        ) : null}

        {!isRemote && isLinux() && (
          <ToggleRow
            icon={<AppWindow className="h-4 w-4 text-amber-500" />}
            title={t("settings.useAppWindowControls")}
            description={t("settings.useAppWindowControlsDescription")}
            checked={!!settings.useAppWindowControls}
            onCheckedChange={(value) =>
              onChange({ useAppWindowControls: value })
            }
          />
        )}
      </div>
    </section>
  );
}

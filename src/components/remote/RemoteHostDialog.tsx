import { useEffect, useState } from "react";
import type { FormEvent, ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { extractErrorMessage } from "@/utils/errorUtils";
import type {
  RemoteAuthMethod,
  RemoteConnectionSecret,
  RemoteHostProfile,
} from "@/lib/api";

type RemoteAuthMode = RemoteAuthMethod["type"];

const DEFAULT_HELPER_PATH = "~/.local/bin/cc-switch-remote-helper";

const AUTH_OPTIONS: Array<{
  type: RemoteAuthMode;
  labelKey: string;
  defaultLabel: string;
}> = [
  {
    type: "sshAgent",
    labelKey: "remote.auth.sshAgent",
    defaultLabel: "SSH Agent",
  },
  {
    type: "keyFile",
    labelKey: "remote.auth.keyFile",
    defaultLabel: "密钥文件",
  },
  {
    type: "password",
    labelKey: "remote.auth.password",
    defaultLabel: "密码",
  },
];

export function RemoteHostDialog({
  open,
  onOpenChange,
  onSave,
  initialProfile,
  initialSecret,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  initialProfile?: RemoteHostProfile;
  initialSecret?: RemoteConnectionSecret;
  onSave: (
    profile: RemoteHostProfile,
    secret?: RemoteConnectionSecret,
  ) => Promise<void> | void;
}) {
  const { t } = useTranslation();
  const [name, setName] = useState("");
  const [host, setHost] = useState("");
  const [port, setPort] = useState("22");
  const [username, setUsername] = useState("");
  const [helperPath, setHelperPath] = useState(DEFAULT_HELPER_PATH);
  const [authMode, setAuthMode] = useState<RemoteAuthMode>("sshAgent");
  const [keyPath, setKeyPath] = useState("~/.ssh/id_ed25519");
  const [password, setPassword] = useState("");
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (!open) return;
    setName(initialProfile?.name ?? "");
    setHost(initialProfile?.host ?? "");
    setPort(String(initialProfile?.port ?? 22));
    setUsername(initialProfile?.username ?? "");
    setHelperPath(initialProfile?.helperPath ?? DEFAULT_HELPER_PATH);
    const authMethod = initialProfile?.authMethod;
    setAuthMode(authMethod?.type ?? "sshAgent");
    setKeyPath(
      authMethod?.type === "keyFile" ? authMethod.path : "~/.ssh/id_ed25519",
    );
    setPassword(initialSecret?.password ?? "");
    setSaving(false);
  }, [open, initialProfile, initialSecret]);

  const buildAuthMethod = (): RemoteAuthMethod => {
    if (authMode === "keyFile") {
      return { type: "keyFile", path: keyPath };
    }
    if (authMode === "password") {
      return { type: "password" };
    }
    return { type: "sshAgent" };
  };

  const handleSubmit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (saving) return;
    const now = Date.now();
    const authMethod = buildAuthMethod();
    setSaving(true);
    try {
      await Promise.resolve(
        onSave(
          {
            id: initialProfile?.id ?? `remote-${now}`,
            name:
              name.trim() ||
              host.trim() ||
              t("remote.defaultName", { defaultValue: "远程服务器" }),
            host: host.trim(),
            port: Number(port) || 22,
            username: username.trim(),
            authMethod,
            helperPath: helperPath.trim() || DEFAULT_HELPER_PATH,
            createdAt: initialProfile?.createdAt ?? now,
            updatedAt: now,
          },
          authMethod.type === "password" && password ? { password } : undefined,
        ),
      );
      onOpenChange(false);
    } catch (error) {
      toast.error(
        t("remote.saveFailed", { defaultValue: "保存远程服务器失败" }),
        {
          description: formatSaveError(error, t),
        },
      );
    } finally {
      setSaving(false);
    }
  };

  const formDisabled = saving;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-xl">
        <form onSubmit={handleSubmit}>
          <DialogHeader>
            <DialogTitle>
              {t("remote.dialog.title", { defaultValue: "远程服务器" })}
            </DialogTitle>
            <DialogDescription>
              {t("remote.dialog.description", {
                defaultValue:
                  "保存远程连接信息。SSH 密码会保存到本机数据库，用于后续连接。",
              })}
            </DialogDescription>
          </DialogHeader>

          <div className="grid gap-5 px-6 py-5">
            <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
              <Field
                id="remote-name"
                label={t("remote.fields.name", { defaultValue: "名称" })}
              >
                <Input
                  id="remote-name"
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                  disabled={formDisabled}
                />
              </Field>
              <Field
                id="remote-host"
                label={t("remote.fields.host", { defaultValue: "主机" })}
              >
                <Input
                  id="remote-host"
                  value={host}
                  onChange={(e) => setHost(e.target.value)}
                  placeholder="10.0.0.10"
                  required
                  disabled={formDisabled}
                />
              </Field>
              <Field
                id="remote-username"
                label={t("remote.fields.username", { defaultValue: "用户名" })}
              >
                <Input
                  id="remote-username"
                  value={username}
                  onChange={(e) => setUsername(e.target.value)}
                  placeholder="deploy"
                  required
                  disabled={formDisabled}
                />
              </Field>
              <Field
                id="remote-port"
                label={t("remote.fields.port", { defaultValue: "端口" })}
              >
                <Input
                  id="remote-port"
                  value={port}
                  onChange={(e) => setPort(e.target.value)}
                  inputMode="numeric"
                  required
                  disabled={formDisabled}
                />
              </Field>
            </div>

            <Field
              id="remote-helper-path"
              label={t("remote.fields.helperPath", {
                defaultValue: "Helper 路径",
              })}
            >
              <Input
                id="remote-helper-path"
                value={helperPath}
                onChange={(e) => setHelperPath(e.target.value)}
                disabled={formDisabled}
              />
            </Field>

            <div className="grid gap-2">
              <Label htmlFor="remote-auth-mode">
                {t("remote.fields.authentication", {
                  defaultValue: "认证方式",
                })}
              </Label>
              <Select
                value={authMode}
                onValueChange={(value) => setAuthMode(value as RemoteAuthMode)}
                disabled={formDisabled}
              >
                <SelectTrigger id="remote-auth-mode">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {AUTH_OPTIONS.map((option) => (
                    <SelectItem key={option.type} value={option.type}>
                      {t(option.labelKey, {
                        defaultValue: option.defaultLabel,
                      })}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>

            {authMode === "keyFile" && (
              <Field
                id="remote-key-path"
                label={t("remote.fields.keyPath", {
                  defaultValue: "SSH 密钥路径",
                })}
              >
                <Input
                  id="remote-key-path"
                  value={keyPath}
                  onChange={(e) => setKeyPath(e.target.value)}
                  required
                  disabled={formDisabled}
                />
              </Field>
            )}

            {authMode === "password" && (
              <Field
                id="remote-password"
                label={t("remote.fields.password", { defaultValue: "密码" })}
              >
                <Input
                  id="remote-password"
                  value={password}
                  onChange={(e) => setPassword(e.target.value)}
                  type="password"
                  placeholder={
                    initialSecret?.password
                      ? t("remote.fields.passwordSavedForSession", {
                          defaultValue: "本次会话已填写",
                        })
                      : undefined
                  }
                  required={!initialSecret?.password}
                  disabled={formDisabled}
                />
              </Field>
            )}
          </div>

          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              onClick={() => onOpenChange(false)}
              disabled={formDisabled}
            >
              {t("common.cancel", { defaultValue: "取消" })}
            </Button>
            <Button type="submit" disabled={formDisabled}>
              {saving
                ? t("remote.saving", { defaultValue: "保存中" })
                : t("common.save", { defaultValue: "保存" })}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}

function formatSaveError(
  error: unknown,
  t: (key: string, options?: any) => string,
) {
  const message = extractErrorMessage(error);
  if (
    message.includes("reading 'invoke'") ||
    message.includes("__TAURI_INTERNALS__") ||
    message.includes("__TAURI__")
  ) {
    return t("remote.errors.tauriUnavailable", {
      defaultValue:
        "当前浏览器预览没有连接桌面端后端，请在 Tauri 应用窗口中测试保存和远程操作。",
    });
  }
  return message || t("remote.errors.unknown", { defaultValue: "未知错误" });
}

function Field({
  id,
  label,
  children,
}: {
  id: string;
  label: string;
  children: ReactNode;
}) {
  return (
    <div className="grid gap-2">
      <Label htmlFor={id}>{label}</Label>
      {children}
    </div>
  );
}

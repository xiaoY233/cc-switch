import { useEffect, useState } from "react";
import type { FormEvent, ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { KeyRound, LockKeyhole, ShieldCheck } from "lucide-react";
import type { LucideIcon } from "lucide-react";
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
import { cn } from "@/lib/utils";
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
  icon: LucideIcon;
}> = [
  {
    type: "sshAgent",
    labelKey: "remote.auth.sshAgent",
    defaultLabel: "SSH Agent",
    icon: ShieldCheck,
  },
  {
    type: "keyFile",
    labelKey: "remote.auth.keyFile",
    defaultLabel: "密钥文件",
    icon: KeyRound,
  },
  {
    type: "password",
    labelKey: "remote.auth.password",
    defaultLabel: "密码",
    icon: LockKeyhole,
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

  const handleSubmit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const now = Date.now();
    const authMethod = buildAuthMethod();
    void Promise.resolve(
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
    ).then(() => onOpenChange(false));
  };

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
                  "保存远程连接信息。密码仅保留在当前会话中，不会写入本地数据库。",
              })}
            </DialogDescription>
          </DialogHeader>

          <div className="grid gap-5 px-6 py-5">
            <div className="grid grid-cols-2 gap-3">
              <Field
                id="remote-name"
                label={t("remote.fields.name", { defaultValue: "名称" })}
              >
                <Input
                  id="remote-name"
                  value={name}
                  onChange={(e) => setName(e.target.value)}
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
              />
            </Field>

            <div className="grid gap-2">
              <Label>
                {t("remote.fields.authentication", {
                  defaultValue: "认证方式",
                })}
              </Label>
              <div className="grid grid-cols-3 gap-2">
                {AUTH_OPTIONS.map((option) => {
                  const Icon = option.icon;
                  const active = authMode === option.type;
                  return (
                    <button
                      key={option.type}
                      type="button"
                      onClick={() => setAuthMode(option.type)}
                      className={cn(
                        "flex h-10 items-center justify-center gap-2 rounded-lg border text-sm transition-colors",
                        active
                          ? "border-primary bg-primary/10 text-primary"
                          : "border-border bg-card text-muted-foreground hover:bg-accent/50 hover:text-foreground",
                      )}
                    >
                      <Icon className="h-4 w-4" />
                      {t(option.labelKey, {
                        defaultValue: option.defaultLabel,
                      })}
                    </button>
                  );
                })}
              </div>
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
                />
              </Field>
            )}
          </div>

          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              onClick={() => onOpenChange(false)}
            >
              {t("common.cancel", { defaultValue: "取消" })}
            </Button>
            <Button type="submit">
              {t("common.save", { defaultValue: "保存" })}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
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

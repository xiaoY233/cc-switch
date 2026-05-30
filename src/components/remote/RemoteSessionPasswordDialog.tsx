import { useEffect, useState } from "react";
import type { FormEvent } from "react";
import { useTranslation } from "react-i18next";
import { LockKeyhole } from "lucide-react";
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
import type { RemoteHostProfile } from "@/lib/api";

export function RemoteSessionPasswordDialog({
  profile,
  onCancel,
  onSubmit,
}: {
  profile: RemoteHostProfile | null;
  onCancel: () => void;
  onSubmit: (password: string) => void;
}) {
  const { t } = useTranslation();
  const [password, setPassword] = useState("");

  useEffect(() => {
    setPassword("");
  }, [profile?.id]);

  const handleSubmit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (!password) return;
    onSubmit(password);
  };

  return (
    <Dialog
      open={Boolean(profile)}
      onOpenChange={(open) => !open && onCancel()}
    >
      <DialogContent
        className="max-w-md"
        data-testid="remote-session-password-dialog"
      >
        <form onSubmit={handleSubmit}>
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <LockKeyhole className="h-5 w-5 text-primary" />
              {t("remote.sessionPassword.title", {
                defaultValue: "输入远程密码",
              })}
            </DialogTitle>
            <DialogDescription>
              {t("remote.sessionPassword.description", {
                defaultValue:
                  "{{name}} 使用密码登录。密码只保存在当前会话中，不会写入本地数据库。",
                name: profile?.name ?? "",
              })}
            </DialogDescription>
          </DialogHeader>

          <div className="grid gap-2 px-6 py-5">
            <Label htmlFor="remote-session-password">
              {t("remote.fields.password", { defaultValue: "密码" })}
            </Label>
            <Input
              id="remote-session-password"
              data-testid="remote-session-password-input"
              value={password}
              onChange={(event) => setPassword(event.target.value)}
              type="password"
              autoFocus
            />
          </div>

          <DialogFooter>
            <Button type="button" variant="outline" onClick={onCancel}>
              {t("common.cancel", { defaultValue: "取消" })}
            </Button>
            <Button
              type="submit"
              disabled={!password}
              data-testid="remote-session-password-confirm"
            >
              {t("common.confirm", { defaultValue: "确定" })}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}

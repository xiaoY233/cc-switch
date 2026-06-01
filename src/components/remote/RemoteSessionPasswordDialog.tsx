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
  onSubmit: (password: string) => void | Promise<void>;
}) {
  const { t } = useTranslation();
  const [password, setPassword] = useState("");
  const [submitting, setSubmitting] = useState(false);

  useEffect(() => {
    setPassword("");
  }, [profile?.id]);

  const handleSubmit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (!password || submitting) return;
    setSubmitting(true);
    try {
      await onSubmit(password);
    } finally {
      setSubmitting(false);
    }
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
                  "{{name}} 使用密码登录。密码会保存到本机数据库，用于后续连接。",
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
              disabled={submitting}
            />
          </div>

          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              onClick={onCancel}
              disabled={submitting}
            >
              {t("common.cancel", { defaultValue: "取消" })}
            </Button>
            <Button
              type="submit"
              disabled={!password || submitting}
              data-testid="remote-session-password-confirm"
            >
              {submitting
                ? t("remote.saving", { defaultValue: "保存中" })
                : t("common.confirm", { defaultValue: "确定" })}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}

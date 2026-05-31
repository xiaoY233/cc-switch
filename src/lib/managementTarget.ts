import type { ManagementTarget } from "@/lib/api";

export const LOCAL_MANAGEMENT_TARGET: ManagementTarget = { type: "local" };

export function getManagementTargetKey(target: ManagementTarget): string {
  return target.type === "remote" ? `remote:${target.profile.id}` : "local";
}

export function shouldReadLocalLiveConfig(
  target: ManagementTarget,
  isProxyTakeover: boolean,
): boolean {
  return target.type === "local" && !isProxyTakeover;
}

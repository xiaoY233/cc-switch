import { invoke } from "@tauri-apps/api/core";
import type { Provider } from "@/types";
import type { AppId } from "./types";
import type { SwitchResult } from "./providers";

export type RemoteAuthMethod =
  | { type: "sshAgent" }
  | { type: "keyFile"; path: string }
  | { type: "password" };

export interface RemoteHostProfile {
  id: string;
  name: string;
  host: string;
  port: number;
  username: string;
  authMethod: RemoteAuthMethod;
  helperPath: string;
  createdAt: number;
  updatedAt: number;
}

export interface RemoteConnectionSecret {
  password?: string;
}

export interface RemoteHealth {
  reachable: boolean;
  helperInstalled: boolean;
  helperVersion?: string;
  platform?: "linux" | "macos" | "unknown";
  capabilities: string[];
  lastError?: string;
}

export type ManagementTarget =
  | { type: "local" }
  | {
      type: "remote";
      profile: RemoteHostProfile;
      secret?: RemoteConnectionSecret;
    };

export const remoteApi = {
  listProfiles(): Promise<RemoteHostProfile[]> {
    return invoke<RemoteHostProfile[]>("remote_list_profiles");
  },

  saveProfile(profile: RemoteHostProfile): Promise<RemoteHostProfile> {
    return invoke<RemoteHostProfile>("remote_save_profile", { profile });
  },

  deleteProfile(id: string): Promise<boolean> {
    return invoke<boolean>("remote_delete_profile", { id });
  },

  validateProfile(profile: RemoteHostProfile): Promise<boolean> {
    return invoke<boolean>("remote_validate_profile", { profile });
  },

  buildStatusCommand(profile: RemoteHostProfile): Promise<string[]> {
    return invoke<string[]>("remote_build_status_command", { profile });
  },

  checkHealth(
    profile: RemoteHostProfile,
    secret?: RemoteConnectionSecret,
  ): Promise<RemoteHealth> {
    return invoke<RemoteHealth>("remote_check_health", { profile, secret });
  },

  getProviders(
    profile: RemoteHostProfile,
    app: AppId,
    secret?: RemoteConnectionSecret,
  ): Promise<Record<string, Provider>> {
    return invoke<Record<string, Provider>>("remote_get_providers", {
      profile,
      app,
      secret,
    });
  },

  getCurrentProvider(
    profile: RemoteHostProfile,
    app: AppId,
    secret?: RemoteConnectionSecret,
  ): Promise<string> {
    return invoke<string>("remote_get_current_provider", {
      profile,
      app,
      secret,
    });
  },

  switchProvider(
    profile: RemoteHostProfile,
    app: AppId,
    id: string,
    secret?: RemoteConnectionSecret,
  ): Promise<SwitchResult> {
    return invoke<SwitchResult>("remote_switch_provider", {
      profile,
      app,
      id,
      secret,
    });
  },

  addProvider(
    profile: RemoteHostProfile,
    app: AppId,
    provider: Provider,
    addToLive?: boolean,
    secret?: RemoteConnectionSecret,
  ): Promise<boolean> {
    return invoke<boolean>("remote_add_provider", {
      profile,
      app,
      provider,
      addToLive,
      secret,
    });
  },

  updateProvider(
    profile: RemoteHostProfile,
    app: AppId,
    provider: Provider,
    originalId?: string,
    secret?: RemoteConnectionSecret,
  ): Promise<boolean> {
    return invoke<boolean>("remote_update_provider", {
      profile,
      app,
      provider,
      originalId,
      secret,
    });
  },

  deleteProvider(
    profile: RemoteHostProfile,
    app: AppId,
    id: string,
    secret?: RemoteConnectionSecret,
  ): Promise<boolean> {
    return invoke<boolean>("remote_delete_provider", {
      profile,
      app,
      id,
      secret,
    });
  },
};

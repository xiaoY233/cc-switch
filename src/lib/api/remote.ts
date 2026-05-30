import { invoke } from "@tauri-apps/api/core";

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

export const remoteApi = {
  validateProfile(profile: RemoteHostProfile): Promise<boolean> {
    return invoke<boolean>("remote_validate_profile", { profile });
  },

  buildStatusCommand(profile: RemoteHostProfile): Promise<string[]> {
    return invoke<string[]>("remote_build_status_command", { profile });
  },
};

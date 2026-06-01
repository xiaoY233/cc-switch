import { invoke } from "@tauri-apps/api/core";
import type { SessionMessage, SessionMeta } from "@/types";
import { remoteApi, type ManagementTarget } from "./remote";

export interface DeleteSessionOptions {
  providerId: string;
  sessionId: string;
  sourcePath: string;
}

export interface DeleteSessionResult extends DeleteSessionOptions {
  success: boolean;
  error?: string;
}

export const sessionsApi = {
  async list(target: ManagementTarget = { type: "local" }): Promise<SessionMeta[]> {
    if (target.type === "remote") {
      return await remoteApi.listSessions(target.profile, target.secret);
    }
    return await invoke("list_sessions");
  },

  async getMessages(
    providerId: string,
    sourcePath: string,
    target: ManagementTarget = { type: "local" },
  ): Promise<SessionMessage[]> {
    if (target.type === "remote") {
      return await remoteApi.getSessionMessages(
        target.profile,
        providerId,
        sourcePath,
        target.secret,
      );
    }
    return await invoke("get_session_messages", { providerId, sourcePath });
  },

  async delete(
    options: DeleteSessionOptions,
    target: ManagementTarget = { type: "local" },
  ): Promise<boolean> {
    if (target.type === "remote") {
      return await remoteApi.deleteSession(
        target.profile,
        options,
        target.secret,
      );
    }
    const { providerId, sessionId, sourcePath } = options;
    return await invoke("delete_session", {
      providerId,
      sessionId,
      sourcePath,
    });
  },

  async deleteMany(
    items: DeleteSessionOptions[],
    target: ManagementTarget = { type: "local" },
  ): Promise<DeleteSessionResult[]> {
    if (target.type === "remote") {
      return await remoteApi.deleteSessions(target.profile, items, target.secret);
    }
    return await invoke("delete_sessions", { items });
  },

  async launchTerminal(options: {
    command: string;
    cwd?: string | null;
    customConfig?: string | null;
  }): Promise<boolean> {
    const { command, cwd, customConfig } = options;
    return await invoke("launch_session_terminal", {
      command,
      cwd,
      customConfig,
    });
  },
};

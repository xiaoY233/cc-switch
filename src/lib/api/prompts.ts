import { invoke } from "@tauri-apps/api/core";
import type { AppId } from "./types";
import { remoteApi, type ManagementTarget } from "./remote";

export interface Prompt {
  id: string;
  name: string;
  content: string;
  description?: string;
  enabled: boolean;
  createdAt?: number;
  updatedAt?: number;
}

export const promptsApi = {
  async getPrompts(
    app: AppId,
    target: ManagementTarget = { type: "local" },
  ): Promise<Record<string, Prompt>> {
    if (target.type === "remote") {
      return await remoteApi.getPrompts(target.profile, app, target.secret);
    }
    return await invoke("get_prompts", { app });
  },

  async upsertPrompt(
    app: AppId,
    id: string,
    prompt: Prompt,
    target: ManagementTarget = { type: "local" },
  ): Promise<void> {
    if (target.type === "remote") {
      return await remoteApi.upsertPrompt(
        target.profile,
        app,
        id,
        prompt,
        target.secret,
      );
    }
    return await invoke("upsert_prompt", { app, id, prompt });
  },

  async deletePrompt(
    app: AppId,
    id: string,
    target: ManagementTarget = { type: "local" },
  ): Promise<void> {
    if (target.type === "remote") {
      return await remoteApi.deletePrompt(target.profile, app, id, target.secret);
    }
    return await invoke("delete_prompt", { app, id });
  },

  async enablePrompt(
    app: AppId,
    id: string,
    target: ManagementTarget = { type: "local" },
  ): Promise<void> {
    if (target.type === "remote") {
      return await remoteApi.enablePrompt(target.profile, app, id, target.secret);
    }
    return await invoke("enable_prompt", { app, id });
  },

  async importFromFile(
    app: AppId,
    target: ManagementTarget = { type: "local" },
  ): Promise<string> {
    if (target.type === "remote") {
      return await remoteApi.importPromptFromFile(
        target.profile,
        app,
        target.secret,
      );
    }
    return await invoke("import_prompt_from_file", { app });
  },

  async getCurrentFileContent(
    app: AppId,
    target: ManagementTarget = { type: "local" },
  ): Promise<string | null> {
    if (target.type === "remote") {
      return await remoteApi.getCurrentPromptFileContent(
        target.profile,
        app,
        target.secret,
      );
    }
    return await invoke("get_current_prompt_file_content", { app });
  },
};

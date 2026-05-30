import { useMutation } from "@tanstack/react-query";
import { remoteApi, type RemoteHostProfile } from "@/lib/api";

export const remoteQueryKeys = {
  all: ["remote"] as const,
  host: (id: string) => ["remote", "host", id] as const,
};

export function useValidateRemoteProfile() {
  return useMutation({
    mutationFn: (profile: RemoteHostProfile) =>
      remoteApi.validateProfile(profile),
  });
}

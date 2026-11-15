// Upload state stays with the captured immutable blobs before model dispatch.
// Explicit preparation retry resumes the existing uploader; uncertainty never calls it.
import type { AttachmentRef } from "../wire/proto";
import { checked, type TrayItem } from "./tray-item";
import { send, type Begun, type UploadCalls } from "./upload";

export interface PreparedFile {
  readonly key: string;
  readonly blob: Blob;
  readonly ref: AttachmentRef;
}

export async function uploadCaptured(
  files: readonly TrayItem[],
  completed: PreparedFile[],
  resumes: Map<string, Begun>,
  calls: UploadCalls,
): Promise<string | null> {
  for (const item of files) {
    if (completed.some((file) => file.key === item.key)) continue;
    let ref: AttachmentRef;
    if (item.state.kind === "ready") ref = item.state.ref;
    else {
      const invalid = await checked(item);
      if (invalid !== null) return invalid;
      const uploaded = await send(
        calls,
        item.file,
        item.name,
        item.mime,
        resumes.get(item.key) ?? null,
        () => {},
        () => false,
      );
      if (uploaded.kind !== "done") {
        if (uploaded.resume !== null) resumes.set(item.key, uploaded.resume);
        return uploaded.kind === "failed"
          ? uploaded.problem.message
          : "Upload stopped.";
      }
      resumes.delete(item.key);
      ref = uploaded.ref;
    }
    completed.push(
      Object.freeze({
        key: item.key,
        blob: item.file,
        ref: Object.freeze({ ...ref }),
      }),
    );
  }
  return null;
}

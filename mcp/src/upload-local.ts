/**
 * Local-file upload for the stdio bridge (SPEC-173).
 * The remote server never sees the path. The bridge PUTs the bytes.
 */
import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";
import path from "node:path";

import { callRemoteTool } from "./gateway-bridge.js";

export function resolveInsideRoot(root: string, file: string): string {
  const base = path.resolve(root);
  const target = path.resolve(base, file);
  const rel = path.relative(base, target);
  if (rel === ".." || rel.startsWith(`..${path.sep}`) || path.isAbsolute(rel)) {
    throw new Error("path escapes the upload root");
  }
  return target;
}

function asRecord(value: unknown): Record<string, unknown> {
  if (value && typeof value === "object") return value as Record<string, unknown>;
  throw new Error("upload tool returned no object");
}

function toolObject(result: {
  content: Array<{ type: string; text?: string }>;
  structuredContent?: unknown;
  isError?: boolean;
}): Record<string, unknown> {
  if (result.isError) {
    throw new Error(result.content?.[0]?.text ?? "upload tool failed");
  }
  if (result.structuredContent) return asRecord(result.structuredContent);
  const text = result.content?.[0]?.text ?? "";
  return asRecord(JSON.parse(text));
}

export async function uploadLocalFile(file: string): Promise<Record<string, unknown>> {
  const root = process.env.EDGEQUAKE_UPLOAD_ROOT ?? process.cwd();
  const absolute = resolveInsideRoot(root, file);
  const bytes = await readFile(absolute);
  const sha256 = createHash("sha256").update(bytes).digest("hex");
  const filename = path.basename(absolute);
  const begin = toolObject(
    await callRemoteTool("eq_upload_begin", {
      filename,
      byte_length: bytes.length,
      sha256,
      media_type: filename.toLowerCase().endsWith(".pdf")
        ? "application/pdf"
        : "application/octet-stream",
    }),
  );
  const url = begin.url;
  const uploadId = begin.upload_id;
  if (typeof url !== "string" || typeof uploadId !== "string") {
    throw new Error("eq_upload_begin did not return a PUT url");
  }
  const headers = new Headers();
  const declared = asRecord(begin.headers ?? {});
  for (const [key, value] of Object.entries(declared)) {
    if (typeof value === "string") headers.set(key, value);
  }
  const put = await fetch(url, { method: "PUT", headers, body: bytes });
  if (!put.ok) {
    throw new Error(`upload PUT failed: ${put.status}`);
  }
  return toolObject(await callRemoteTool("eq_upload_commit", { upload_id: uploadId }));
}

// src/lib/file-save.ts
import { save } from "@tauri-apps/plugin-dialog";
import { writeFile } from "@tauri-apps/plugin-fs";

/**
 * Single save pathway for every export in the app (inventory, reports, …).
 *
 * Opens a native "Save As" dialog, then writes the bytes to the chosen path.
 * Returns the written path, or null when the user cancelled the dialog.
 */
export async function saveWorkbook(
  defaultName: string,
  data: Uint8Array,
): Promise<string | null> {
  const path = await save({
    defaultPath: defaultName,
    filters: [{ name: "Excel workbook", extensions: ["xlsx"] }],
  });
  if (!path) return null;
  await writeFile(path, data);
  return path;
}

/** Standard moon-pipe filename stamp used by all exports. */
export const fileStamp = () => new Date().toISOString().slice(0, 10);

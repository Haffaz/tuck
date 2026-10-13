/** Path helpers for folder names such as "Finance/Payslips". */
import type { Rule } from "./types";

export const nameOf = (path: string) =>
  path.split("/").filter(Boolean).pop() || path;
export const plural = (n: number, word: string) =>
  `${n} ${word}${n === 1 ? "" : "s"}`;

/** Folder paths in display order: each parent followed by its children. */
export function ordered(rules: Rule[]): Rule[] {
  const parents = rules.filter((r) => !r.folder.includes("/"));
  const orphans = rules.filter(
    (r) =>
      r.folder.includes("/") &&
      !parents.some((p) => r.folder.startsWith(`${p.folder}/`)),
  );
  return [
    ...parents.flatMap((p) => [
      p,
      ...rules.filter((r) => r.folder.startsWith(`${p.folder}/`)),
    ]),
    ...orphans,
  ];
}

export const parentOf = (path: string) =>
  path.includes("/") ? path.slice(0, path.lastIndexOf("/") + 1) : "";

/** True when `path` is `folder` itself or inside it. */
export const isWithin = (path: string, folder: string) =>
  path === folder || path.startsWith(`${folder}/`);

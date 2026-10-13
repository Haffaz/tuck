import type { CSSProperties } from "react";
import {
  Briefcase,
  Car,
  Folder,
  GraduationCap,
  HeartPulse,
  House,
  IdCard,
  Image,
  Landmark,
  type LucideIcon,
  Plane,
  Receipt,
  Scale,
  ShieldCheck,
  Wallet,
  Wrench,
} from "lucide-react";

/** Keyword → icon and tint. The first match wins, so specific words come first. */
const looks: [RegExp, LucideIcon, string][] = [
  [/tax|bank|statement|financ|money|account|budget/i, Wallet, "#16a34a"],
  [/invoice|receipt|bill|payment|purchase/i, Receipt, "#d97706"],
  [/work|employ|pay|salary|job|career|contract/i, Briefcase, "#7c3aed"],
  [/home|hous|rent|lease|utilit|propert/i, House, "#ea580c"],
  [/health|medic|clinic|doctor|lab|prescri/i, HeartPulse, "#e11d48"],
  [/insur|cover|polic|warrant/i, ShieldCheck, "#0891b2"],
  [/ident|passport|immigra|visa|residen|licen/i, IdCard, "#4f46e5"],
  [
    /educat|school|universit|transcript|diplom|academ|certif/i,
    GraduationCap,
    "#2563eb",
  ],
  [/travel|flight|hotel|trip|booking|ticket/i, Plane, "#0d9488"],
  [/car|vehicle|motor/i, Car, "#64748b"],
  [/legal|court|law/i, Scale, "#92400e"],
  [/repair|maint|equip/i, Wrench, "#57534e"],
  [/photo|image|picture/i, Image, "#c026d3"],
  [/gov|council|state/i, Landmark, "#475569"],
];
const fallbackTints = [
  "#2563eb",
  "#0d9488",
  "#7c3aed",
  "#d97706",
  "#db2777",
  "#4f46e5",
];

export function folderLook(path: string): { Icon: LucideIcon; tint: string } {
  for (const [pattern, Icon, tint] of looks)
    if (pattern.test(path)) return { Icon, tint };
  const hash = [...path].reduce((h, c) => (h * 31 + c.charCodeAt(0)) >>> 0, 7);
  return { Icon: Folder, tint: fallbackTints[hash % fallbackTints.length] };
}

export const tint = (color: string) => ({ "--tint": color }) as CSSProperties;

/** A rounded, tinted icon tile for a folder. */
export function FolderTile({
  path,
  size = 30,
}: {
  path: string;
  size?: number;
}) {
  const { Icon, tint: color } = folderLook(path);
  return (
    <span
      className="tile"
      style={{ ...tint(color), width: size, height: size }}
      aria-hidden="true"
    >
      <Icon size={Math.round(size * 0.52)} strokeWidth={1.9} />
    </span>
  );
}

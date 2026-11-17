"use client";

import { useSyncExternalStore } from "react";
import { ArrowDown } from "lucide-react";
import { buttonVariants } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { downloads } from "@/content/site";

type OS = "windows" | "linux" | "other";

function detect(): OS {
  const nav = navigator as Navigator & { userAgentData?: { platform?: string } };
  const p = (nav.userAgentData?.platform || nav.platform || nav.userAgent).toLowerCase();
  if (p.includes("win")) return "windows";
  if (p.includes("linux") && !p.includes("android")) return "linux";
  return "other";
}

const subscribe = () => () => {};

function primaryFile(os: "Windows" | "Linux") {
  const f = downloads.find((d) => d.os === os)?.files.find((x) => x.primary);
  return f ? { href: f.href, size: f.size } : { href: "#descargar", size: "" };
}

const win = primaryFile("Windows");
const lin = primaryFile("Linux");

const targets: Record<OS, { label: string; href: string; note: string }> = {
  windows: { label: "Descargar para Windows", href: win.href, note: `Instalador · ${win.size}` },
  linux: { label: "Descargar para Linux", href: lin.href, note: `Autoejecutable x86‑64 · ${lin.size}` },
  other: { label: "Descargar gratis", href: "#descargar", note: "Windows y Linux" },
};

/** Primary call to action: offers the build for the visitor's OS. */
export function DownloadButton({ className, extra }: { className?: string; extra?: string }) {
  const os = useSyncExternalStore<OS>(subscribe, detect, () => "other");
  const t = targets[os];
  return (
    <div className={cn("flex flex-col items-center gap-2", className)}>
      <a
        href={t.href}
        className={cn(
          buttonVariants({ size: "lg" }),
          "h-12 rounded-full px-6 text-[15px] font-medium transition-[background-color,transform] duration-200 hover:bg-brand",
        )}
      >
        {t.label}
        <ArrowDown data-icon="inline-end" className="size-4" aria-hidden="true" />
      </a>
      <span className="tabular text-xs text-muted-foreground">
        {t.note}
        {extra && <> · {extra}</>}
      </span>
    </div>
  );
}

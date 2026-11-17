import { cn } from "@/lib/utils";

/** Soryntel Chemistry logo (vectorized from design/logo-source.png). */
export function Logo({ className, variant = "auto" }: { className?: string; variant?: "auto" | "white" }) {
  const common = "h-full w-auto";
  return (
    <span className={cn("inline-flex h-9 shrink-0", className)}>
      {/* eslint-disable @next/next/no-img-element -- static SVG brand asset */}
      {variant === "white" ? (
        <img src="/brand/soryntel-chemistry-logo-white.svg" alt="Soryntel Chemistry" width={905} height={325} className={common} />
      ) : (
        <>
          <img src="/brand/soryntel-chemistry-logo.svg" alt="Soryntel Chemistry" width={905} height={325} className={cn(common, "dark:hidden")} />
          <img src="/brand/soryntel-chemistry-logo-white.svg" alt="" aria-hidden="true" width={905} height={325} className={cn(common, "hidden dark:block")} />
        </>
      )}
      {/* eslint-enable @next/next/no-img-element */}
    </span>
  );
}

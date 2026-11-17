import { cn } from "@/lib/utils";

/** Frame that presents a real recording or capture of the desktop app. */
export function AppWindow({
  title,
  className,
  children,
  footer,
}: {
  title: string;
  className?: string;
  children: React.ReactNode;
  footer?: React.ReactNode;
}) {
  return (
    <figure
      className={cn(
        "relative m-0 overflow-hidden rounded-[14px] bg-window shadow-[var(--shadow-window)]",
        className,
      )}
    >
      <div className="flex h-9 items-center gap-2 border-b border-border bg-card px-3.5" aria-hidden="true">
        <span className="size-2.5 rounded-full bg-foreground/15" />
        <span className="size-2.5 rounded-full bg-foreground/15" />
        <span className="size-2.5 rounded-full bg-foreground/15" />
        <span className="ml-3 truncate text-xs text-muted-foreground">Soryntel Chemistry — {title}</span>
      </div>
      {children}
      {footer}
    </figure>
  );
}

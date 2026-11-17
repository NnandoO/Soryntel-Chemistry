"use client";

import { useEffect, useState } from "react";
import { Logo } from "@/components/logo";
import { buttonVariants } from "@/components/ui/button";
import { cn } from "@/lib/utils";

const links = [
  { href: "#en-accion", label: "En acción" },
  { href: "#modulos", label: "Módulos" },
  { href: "#validacion", label: "Validación" },
];

export function SiteHeader() {
  const [scrolled, setScrolled] = useState(false);
  useEffect(() => {
    const on = () => setScrolled(window.scrollY > 8);
    on();
    window.addEventListener("scroll", on, { passive: true });
    return () => window.removeEventListener("scroll", on);
  }, []);

  return (
    <header
      className={cn(
        "sticky top-0 z-30 border-b bg-background/80 backdrop-blur-md backdrop-saturate-150 transition-colors duration-200",
        scrolled ? "border-border" : "border-transparent",
      )}
    >
      <div className="mx-auto flex h-16 w-[min(100%-2rem,72rem)] items-center gap-7">
        <a href="#inicio" aria-label="Soryntel Chemistry, inicio" className="flex items-center gap-3 whitespace-nowrap text-[15px] tracking-[-0.01em]">
          <Logo className="h-11" />
          <span className="hidden border-l border-border pl-3 text-muted-foreground lg:inline">Soryntel Chemistry</span>
        </a>
        <nav aria-label="Navegación principal" className="ml-auto hidden gap-7 md:flex">
          {links.map((l) => (
            <a key={l.href} href={l.href} className="text-sm text-muted-foreground transition-colors hover:text-foreground">
              {l.label}
            </a>
          ))}
        </nav>
        <a
          href="#descargar"
          className={cn(buttonVariants({ size: "sm" }), "ml-auto h-8 rounded-full px-4 text-sm hover:bg-brand md:ml-0")}
        >
          Descargar
        </a>
      </div>
    </header>
  );
}

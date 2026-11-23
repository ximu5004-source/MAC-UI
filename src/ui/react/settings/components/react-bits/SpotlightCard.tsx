import { type ReactNode, useRef } from "react";
import cs from "./SpotlightCard.module.css";

// Adapted from React Bits SpotlightCard by David Haz. See LICENSE.md.
export function SpotlightCard({ children, className = "" }: { children: ReactNode; className?: string }) {
  const card = useRef<HTMLDivElement>(null);
  return (
    <div
      ref={card}
      className={`${cs.card} ${className}`}
      onPointerMove={(event) => {
        if (event.pointerType === "touch") return;
        const rect = event.currentTarget.getBoundingClientRect();
        card.current?.style.setProperty("--spotlight-x", `${event.clientX - rect.left}px`);
        card.current?.style.setProperty("--spotlight-y", `${event.clientY - rect.top}px`);
      }}
    >
      {children}
    </div>
  );
}

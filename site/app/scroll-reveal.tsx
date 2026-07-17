"use client";

import { useEffect } from "react";

/**
 * Fades sections in as they scroll into view. A no-op for anyone who has asked their system
 * to reduce motion, and for anyone without IntersectionObserver, the content is already
 * visible because the "shown" class is all this adds.
 */
export default function ScrollReveal() {
  useEffect(() => {
    const reduced = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    const targets = document.querySelectorAll<HTMLElement>(".reveal");

    // Without JS the content is already visible; only arm the hidden-then-fade state when we
    // can also reveal it. Anyone who prefers reduced motion keeps it visible and still.
    if (reduced || !("IntersectionObserver" in window)) {
      return;
    }

    document.documentElement.classList.add("reveal-armed");

    const observer = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          if (entry.isIntersecting) {
            entry.target.classList.add("shown");
            observer.unobserve(entry.target);
          }
        }
      },
      { rootMargin: "0px 0px -10% 0px" },
    );

    targets.forEach((el) => observer.observe(el));
    return () => observer.disconnect();
  }, []);

  return null;
}

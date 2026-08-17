import { useEffect, useRef, useState, type ReactNode } from "react";
import type { Track } from "../../../domain/models";
import { usePlaybackPort } from "../../../shared/api/GatewayProvider";

export function TrackCover({ track, className, children, disabled = false }: {
  track: Track | null;
  className: string;
  children: ReactNode;
  disabled?: boolean;
}) {
  const playback = usePlaybackPort();
  const host = useRef<HTMLSpanElement>(null);
  const [visible, setVisible] = useState(false);
  const [url, setUrl] = useState<string | null>(null);

  useEffect(() => {
    setUrl(null);
    setVisible(false);
    const element = host.current;
    if (!element || disabled || !track) return;
    if (!("IntersectionObserver" in window)) {
      setVisible(true);
      return;
    }
    const observer = new IntersectionObserver((entries) => {
      if (entries.some((entry) => entry.isIntersecting)) {
        setVisible(true);
        observer.disconnect();
      }
    }, { rootMargin: "80px" });
    observer.observe(element);
    return () => observer.disconnect();
  }, [track?.id, disabled]);

  useEffect(() => {
    let active = true;
    if (!visible || !track || disabled || !playback) return () => { active = false; };
    void playback.getTrackCover(track.id)
      .then((nextUrl) => { if (active) setUrl(nextUrl); })
      .catch(() => { if (active) setUrl(null); });
    return () => { active = false; };
  }, [disabled, playback, track?.id, visible]);

  return <span ref={host} className={className} style={{ background: track?.coverColor ?? "#333" }}>{url ? <img src={url} alt="" draggable={false} /> : children}</span>;
}

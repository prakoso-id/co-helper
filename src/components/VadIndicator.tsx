import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { useChatStore } from "../store/chat";

const statusColors: Record<string, string> = {
  idle: "bg-gray-500",
  listening: "bg-blue-500 animate-pulse",
  processing: "bg-yellow-500 animate-pulse",
};

const statusLabels: Record<string, string> = {
  idle: "Idle",
  listening: "Listening",
  processing: "Processing",
};

export function VadIndicator() {
  const { status, setStatus } = useChatStore();

  useEffect(() => {
    const unlisten = listen<string>("vad_status", (e) => {
      setStatus(e.payload as "idle" | "listening" | "processing");
    });
    return () => {
      unlisten.then((f) => f());
    };
  }, []);

  return (
    <div className="flex items-center gap-1.5 ml-3">
      <div className={`w-2.5 h-2.5 rounded-full ${statusColors[status]}`} />
      <span className="text-xs text-gray-400">{statusLabels[status]}</span>
    </div>
  );
}

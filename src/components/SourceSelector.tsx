import { useChatStore } from "../store/chat";
import { invoke } from "@tauri-apps/api/core";

export function SourceSelector() {
  const { sources, setSources } = useChatStore();

  const toggle = (source: "mic" | "system") => {
    const next = sources.includes(source)
      ? sources.filter((s) => s !== source)
      : [...sources, source];
    setSources(next);
    invoke("start_capture", { sources: next }).catch(() => {});
  };

  const btn = (active: boolean, label: string, onClick: () => void) => (
    <button
      onClick={onClick}
      className={`px-2 py-1 text-xs rounded transition-colors ${
        active
          ? "bg-[#e94560] text-white"
          : "bg-[#1a1a2e] text-gray-400 hover:text-gray-200"
      }`}
    >
      {label}
    </button>
  );

  return (
    <div className="flex gap-1">
      {btn(sources.includes("mic"), "Mic", () => toggle("mic"))}
      {btn(sources.includes("system"), "System", () => toggle("system"))}
    </div>
  );
}

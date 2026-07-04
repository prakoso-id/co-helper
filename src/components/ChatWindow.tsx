import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useChatStore, type ChatMessage } from "../store/chat";
import { MessageBubble } from "./MessageBubble";

export function ChatWindow() {
  const { messages, addMessage, appendAssistantToken, setStreaming, clearMessages } = useChatStore();
  const [input, setInput] = useState("");
  const scrollRef = useRef<HTMLDivElement>(null);

  // Subscribe to Tauri events
  useEffect(() => {
    const unlisten = Promise.all([
      listen<{ text: string; source: "mic" | "system"; segmentId: string }>(
        "transcript",
        (e) => {
          const role = e.payload.source === "mic" ? "user" : "meeting";
          addMessage({
            id: e.payload.segmentId,
            role,
            content: e.payload.text,
            timestamp: Date.now(),
          });
        }
      ),
      listen<string>("llm_token", (e) => {
        appendAssistantToken(e.payload);
      }),
      listen<{ segmentId: string; fullText: string }>("llm_end", (e) => {
        useChatStore.getState().finishStreaming(e.payload.segmentId, e.payload.fullText);
      }),
      listen<{ segmentId: string }>("llm_start", (e) => {
        addMessage({
          id: e.payload.segmentId,
          role: "assistant",
          content: "",
          streaming: true,
          timestamp: Date.now(),
        });
        setStreaming(true);
      }),
    ]);

    return () => {
      unlisten.then((fns) => fns.forEach((f) => f()));
    };
  }, []);

  // Auto-scroll
  useEffect(() => {
    scrollRef.current?.scrollTo({ top: scrollRef.current.scrollHeight, behavior: "smooth" });
  }, [messages]);

  const sendText = async () => {
    if (!input.trim()) return;
    const msg: ChatMessage = {
      id: Math.random().toString(36).substring(2, 11),
      role: "user",
      content: input.trim(),
      timestamp: Date.now(),
    };
    addMessage(msg);
    setInput("");
    await invoke("send_to_llm", { messages: [{ role: "user", content: msg.content }] });
  };

  return (
    <div className="flex flex-col h-full">
      {/* Messages */}
      <div ref={scrollRef} className="flex-1 overflow-y-auto px-3 py-4 space-y-3">
        {messages.length === 0 && (
          <div className="text-center text-gray-500 mt-8 text-sm">
            No messages yet. Type below or start listening.
          </div>
        )}
        {messages.map((m) => (
          <MessageBubble key={m.id} message={m} />
        ))}
      </div>

      {/* Input */}
      <div className="flex gap-2 px-3 py-3 bg-[#16213e] border-t border-[#0f3460]">
        <button
          onClick={clearMessages}
          className="px-2 text-xs text-gray-500 hover:text-gray-300"
          title="Clear chat"
        >
          ✕
        </button>
        <input
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && !e.shiftKey && sendText()}
          placeholder="Type or wait for transcript..."
          className="flex-1 bg-[#1a1a2e] text-gray-100 px-3 py-2 rounded-lg border border-[#0f3460] focus:outline-none focus:border-[#e94560] text-sm"
        />
        <button
          onClick={sendText}
          className="px-4 py-2 bg-[#e94560] text-white rounded-lg text-sm font-medium hover:bg-[#c81e45] transition-colors"
        >
          Send
        </button>
      </div>
    </div>
  );
}

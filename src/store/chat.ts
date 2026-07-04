import { create } from "zustand";

export type MessageRole = "user" | "assistant" | "meeting";
export type VadStatus = "idle" | "listening" | "processing";

export interface ChatMessage {
  id: string;
  role: MessageRole;
  content: string;
  streaming?: boolean;
  timestamp: number;
}

interface ChatState {
  messages: ChatMessage[];
  status: VadStatus;
  isStreaming: boolean;
  sources: ("mic" | "system")[];

  addMessage: (msg: ChatMessage) => void;
  appendAssistantToken: (token: string) => void;
  finishStreaming: (id: string, fullText: string) => void;
  setStatus: (status: VadStatus) => void;
  setStreaming: (val: boolean) => void;
  setSources: (sources: ("mic" | "system")[]) => void;
  clearMessages: () => void;
}

export const useChatStore = create<ChatState>((set) => ({
  messages: [],
  status: "idle",
  isStreaming: false,
  sources: ["mic", "system"],

  addMessage: (msg) => set((s) => ({ messages: [...s.messages, msg] })),

  appendAssistantToken: (token) =>
    set((s) => {
      const msgs = [...s.messages];
      const last = msgs[msgs.length - 1];
      if (last?.streaming && last.role === "assistant") {
        msgs[msgs.length - 1] = {
          ...last,
          content: last.content + token,
        };
      }
      return { messages: msgs };
    }),

  finishStreaming: (id, fullText) =>
    set((s) => ({
      messages: s.messages.map((m) =>
        m.id === id ? { ...m, content: fullText, streaming: false } : m
      ),
      isStreaming: false,
    })),

  setStatus: (status) => set({ status }),
  setStreaming: (val) => set({ isStreaming: val }),
  setSources: (sources) => set({ sources }),
  clearMessages: () => set({ messages: [] }),
}));

import type { ChatMessage } from "../store/chat";

const roleStyles: Record<string, string> = {
  user: "bg-[#0f3460] text-blue-100 ml-4",
  assistant: "bg-[#1a1a2e] text-green-100 mr-4 border border-[#0f3460]",
  meeting: "bg-[#2d1b3d] text-purple-100 ml-4",
};

const roleLabels: Record<string, string> = {
  user: "You",
  assistant: "AI",
  meeting: "Meeting",
};

export function MessageBubble({ message }: { message: ChatMessage }) {
  return (
    <div className="space-y-1">
      <div className="text-xs text-gray-500 px-2">
        {roleLabels[message.role]}
        {" · "}
        {new Date(message.timestamp).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })}
      </div>
      <div className={`rounded-lg px-3 py-2 text-sm ${roleStyles[message.role]}`}>
        {message.content || (message.streaming ? "..." : "")}
        {message.streaming && (
          <span className="inline-block w-2 h-4 ml-1 bg-green-400 animate-pulse" />
        )}
      </div>
    </div>
  );
}

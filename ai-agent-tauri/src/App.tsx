import { useEffect, useRef, useState } from "preact/hooks";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";
import { listen } from "@tauri-apps/api/event";
import Markdown from "markdown-to-jsx";

interface Message {
  role: "user" | "assistant" | "system";
  content: string;
  timestamp: Date;
  toolCalls?: any;
}

interface AgentChunk {
  delta?: string;
  tool_calls?: any;
}

function App() {
  const [messages, setMessages] = useState<Message[]>([]);
  const [inputText, setInputText] = useState("");
  const [isLoading, setIsLoading] = useState(false);
  const [currentAssistantMessage, setCurrentAssistantMessage] = useState("");
  const messagesEndRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const scrollToBottom = () => {
      messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
    };
    scrollToBottom();
  }, [messages, currentAssistantMessage]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;

    const setupListener = async () => {
      console.log("Setting up event listener for agent-chunk");
      unlisten = await listen<AgentChunk>("agent-chunk", (event) => {
        const chunk = event.payload;
        console.log("Received chunk:", chunk);

        if (chunk.delta) {
          console.log("Delta content:", chunk.delta);
          // Accumulate the text chunks without stopping loading
          setCurrentAssistantMessage((prev) => prev + chunk.delta);
          // Don't set isLoading to false here - let it stream
        }

        if (chunk.tool_calls) {
          console.log("Tool calls:", chunk.tool_calls);
          setMessages((prev) => [
            ...prev,
            {
              role: "system",
              content: `Tool called: ${JSON.stringify(chunk.tool_calls, null, 2)}`,
              timestamp: new Date(),
              toolCalls: chunk.tool_calls,
            },
          ]);
        }
      });
      console.log("Event listener set up successfully");
    };

    setupListener().catch(err => {
      console.error("Failed to setup listener:", err);
    });

    return () => {
      if (unlisten) {
        unlisten();
      }
    };
  }, []);

  useEffect(() => {
    if (currentAssistantMessage && !isLoading) {
      setMessages((prev) => [
        ...prev,
        {
          role: "assistant",
          content: currentAssistantMessage,
          timestamp: new Date(),
        },
      ]);
      setCurrentAssistantMessage("");
    }
  }, [isLoading, currentAssistantMessage]);

  const handleSendMessage = async () => {
    if (!inputText.trim() || isLoading) return;

    const userMessage: Message = {
      role: "user",
      content: inputText,
      timestamp: new Date(),
    };

    setMessages((prev) => [...prev, userMessage]);
    const messageText = inputText;
    setInputText("");
    setIsLoading(true);
    setCurrentAssistantMessage("");

    try {
      console.log("Sending message:", messageText);
      await invoke("chat_with_agent", { message: messageText });
      console.log("Invoke completed - stream finished");

      // Stream is complete, stop loading
      setIsLoading(false);
    } catch (error) {
      console.error("Error invoking chat_with_agent:", error);
      setIsLoading(false);
      setMessages((prev) => [
        ...prev,
        {
          role: "system",
          content: `Error: ${error instanceof Error ? error.message : String(error)}`,
          timestamp: new Date(),
        },
      ]);
    }
  };

  const handleKeyPress = (e: KeyboardEvent) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSendMessage();
    }
  };


  const autoResizeTextarea = (e: Event) => {
    const textarea = e.target as HTMLTextAreaElement;
    textarea.style.height = 'auto';
    textarea.style.height = Math.min(textarea.scrollHeight, 120) + 'px';
  };

  return (
    <div class="h-screen flex flex-col bg-gradient-to-br from-slate-50 to-slate-100">
      {/* Header */}
      <header class="fixed top-0 left-0 right-0 z-10 bg-white/80 backdrop-blur-md border-b border-slate-200/60 px-6 py-4 flex items-center justify-between shadow-sm">
        <div class="flex items-center space-x-4">
          <div class="w-10 h-10 rounded-xl bg-gradient-to-br from-violet-500 to-purple-600 flex items-center justify-center shadow-lg">
            <span class="text-white text-sm font-bold">AI</span>
          </div>
          <div>
            <h1 class="text-xl font-bold bg-gradient-to-r from-violet-600 to-purple-600 bg-clip-text text-transparent">AI Agent</h1>
            <p class="text-xs text-slate-500 flex items-center gap-1">
              <span class="w-2 h-2 rounded-full bg-green-500 animate-pulse"></span>
              {isLoading ? "Thinking..." : "Ready"}
            </p>
          </div>
        </div>
        <button class="p-2 hover:bg-slate-100 rounded-lg transition-colors">
          <svg class="w-5 h-5 text-slate-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 5v.01M12 12v.01M12 19v.01M12 6a1 1 0 110-2 1 1 0 010 2zm0 7a1 1 0 110-2 1 1 0 010 2zm0 7a1 1 0 110-2 1 1 0 010 2z"></path>
          </svg>
        </button>
      </header>

      {/* Conversation Area */}
      <main class="flex-1 overflow-y-auto custom-scrollbar px-6 py-6 space-y-5 mt-[76px] mb-[160px]">
        {messages.length === 0 && (
          <div class="flex flex-col items-center justify-center h-full space-y-4 opacity-60">
            <div class="w-16 h-16 rounded-2xl bg-gradient-to-br from-violet-500 to-purple-600 flex items-center justify-center shadow-xl">
              <svg class="w-8 h-8 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 10h.01M12 10h.01M16 10h.01M9 16H5a2 2 0 01-2-2V6a2 2 0 012-2h14a2 2 0 012 2v8a2 2 0 01-2 2h-5l-5 5v-5z"></path>
              </svg>
            </div>
            <p class="text-slate-400 text-sm">Start a conversation with the AI agent</p>
          </div>
        )}

        {messages.map((message, index) => (
          <div key={index} class={`flex items-start gap-3 message-animate ${message.role === "user" ? "flex-row-reverse" : ""}`}>
            <div class={`w-10 h-10 rounded-xl flex-shrink-0 flex items-center justify-center shadow-lg ${
              message.role === "user"
                ? "bg-gradient-to-br from-emerald-400 to-emerald-600"
                : message.role === "assistant"
                  ? "bg-gradient-to-br from-violet-500 to-purple-600"
                  : "bg-gradient-to-br from-amber-400 to-orange-500"
            }`}>
              <span class="text-white text-xs font-bold">
                {message.role === "user" ? "You" : message.role === "assistant" ? "AI" : "!"}
              </span>
            </div>
            <div class="flex flex-col max-w-[70%] space-y-1">
              <div class={`rounded-2xl px-4 py-3 shadow-md ${
                message.role === "user"
                  ? "bg-gradient-to-br from-emerald-500 to-emerald-600 text-white"
                  : message.role === "assistant"
                    ? "bg-white text-slate-800 border border-slate-200"
                    : "bg-amber-50 border border-amber-200 text-amber-900"
              }`}>
                {message.role === "system" && (
                  <div class="text-xs font-semibold mb-2 opacity-75">System Message</div>
                )}
                <div class="text-[15px] leading-relaxed markdown-content">
                  <Markdown>{message.content}</Markdown>
                </div>
              </div>
              <span class={`text-xs text-slate-400 px-2 ${message.role === "user" ? "text-right" : "text-left"}`}>
                {message.timestamp.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}
              </span>
            </div>
          </div>
        ))}

        {/* Current streaming assistant message */}
        {isLoading && currentAssistantMessage && (
          <div class="flex items-start gap-3 message-animate">
            <div class="w-10 h-10 rounded-xl bg-gradient-to-br from-violet-500 to-purple-600 flex-shrink-0 flex items-center justify-center shadow-lg">
              <span class="text-white text-xs font-bold">AI</span>
            </div>
            <div class="flex flex-col max-w-[70%] space-y-1">
              <div class="bg-white text-slate-800 rounded-2xl px-4 py-3 shadow-md border border-slate-200">
                <div class="text-[15px] leading-relaxed markdown-content">
                  <Markdown>{currentAssistantMessage}</Markdown>
                </div>
                <span class="inline-block w-2 h-4 bg-violet-500 ml-1 animate-pulse"></span>
              </div>
              <span class="text-xs text-slate-400 px-2">streaming...</span>
            </div>
          </div>
        )}

        {/* Typing indicator */}
        {isLoading && !currentAssistantMessage && (
          <div class="flex items-start gap-3 message-animate">
            <div class="w-10 h-10 rounded-xl bg-gradient-to-br from-violet-500 to-purple-600 flex-shrink-0 flex items-center justify-center shadow-lg">
              <span class="text-white text-xs font-bold">AI</span>
            </div>
            <div class="flex flex-col max-w-[70%] space-y-1">
              <div class="bg-white text-slate-800 rounded-2xl px-4 py-3 shadow-md border border-slate-200">
                <div class="flex gap-1.5">
                  <div class="w-2.5 h-2.5 bg-violet-400 rounded-full typing-dot"></div>
                  <div class="w-2.5 h-2.5 bg-violet-400 rounded-full typing-dot"></div>
                  <div class="w-2.5 h-2.5 bg-violet-400 rounded-full typing-dot"></div>
                </div>
              </div>
              <span class="text-xs text-slate-400 px-2">thinking...</span>
            </div>
          </div>
        )}
        <div ref={messagesEndRef} />
      </main>

      {/* Input Area */}
      <div class="fixed bottom-0 left-0 right-0 z-10 bg-white/80 backdrop-blur-md border-t border-slate-200/60 px-6 py-4 shadow-lg">
        {/* Quick Actions */}
        <div class="flex gap-2 mb-3 overflow-x-auto pb-1">
          <button
            onClick={() => setInputText("What's the current time?")}
            class="px-4 py-2 bg-slate-100 text-slate-700 rounded-xl text-xs font-medium whitespace-nowrap hover:bg-slate-200 transition-all hover:shadow-md flex items-center gap-2"
          >
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z"></path>
            </svg>
            Ask Time
          </button>
          <button
            onClick={() => setInputText("Search the web for AI news")}
            class="px-4 py-2 bg-slate-100 text-slate-700 rounded-xl text-xs font-medium whitespace-nowrap hover:bg-slate-200 transition-all hover:shadow-md flex items-center gap-2"
          >
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path>
            </svg>
            Search the web
          </button>
          <button
            onClick={() => setInputText("What tools can you use?")}
            class="px-4 py-2 bg-slate-100 text-slate-700 rounded-xl text-xs font-medium whitespace-nowrap hover:bg-slate-200 transition-all hover:shadow-md flex items-center gap-2"
          >
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8.228 9c.549-1.165 2.03-2 3.772-2 2.21 0 4 1.343 4 3 0 1.4-1.278 2.575-3.006 2.907-.542.104-.994.54-.994 1.093m0 3h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path>
            </svg>
         List tools
          </button>
        </div>

        {/* Input Field */}
        <div class="flex items-end gap-3">
          <div class="flex-1 relative">
            <textarea
              value={inputText}
              onInput={(e) => {
                setInputText(e.currentTarget.value);
                autoResizeTextarea(e);
              }}
              onKeyDown={handleKeyPress}
              placeholder="Type your message..."
              class="w-full px-4 py-3 bg-slate-50 border border-slate-200 rounded-2xl resize-none focus:outline-none focus:ring-2 focus:ring-violet-500 focus:border-transparent text-[15px] placeholder-slate-400 transition-all"
              rows={1}
              style="min-height: 52px; max-height: 120px;"
              disabled={isLoading}
            />
          </div>
          <button
            onClick={handleSendMessage}
            disabled={!inputText.trim() || isLoading}
            class="h-[52px] w-[52px] bg-gradient-to-br from-violet-500 to-purple-600 text-white rounded-2xl hover:shadow-lg transition-all duration-200 disabled:opacity-40 disabled:cursor-not-allowed flex items-center justify-center flex-shrink-0"
          >
            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 19l9 2-9-18-9 18 9-2zm0 0v-8"></path>
            </svg>
          </button>
        </div>
      </div>
    </div>
  );
}

export default App;
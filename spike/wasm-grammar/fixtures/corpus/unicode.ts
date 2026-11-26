const café = "café";
const 日本語 = "こんにちは";
const emoji = "🚀";
const escaped = "\u{1F680}";

// Ünïcödé comment with 🚀

export function greet(name: string): string {
  return `Hello, ${name}! ${café} ${日本語} ${emoji} ${escaped}`;
}

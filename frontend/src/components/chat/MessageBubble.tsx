import { clsx } from 'clsx';
import { Copy, CheckCheck, Sparkles } from 'lucide-react';
import ReactMarkdown from 'react-markdown';
import oneDark from 'react-syntax-highlighter/dist/esm/styles/prism/one-dark';
import { useState, useCallback, Suspense, lazy } from 'react';
import { useToast } from '../../components/ui/Toast';
import { formatTime } from '../../utils/helpers';
import type { Message } from '../../types';

// Lazy-loaded: Prism bundles every language (~hundreds of KB), so it ships
// in its own chunk only when a message actually contains a code block.
const SyntaxHighlighter = lazy(() =>
  import('react-syntax-highlighter').then((m) => ({ default: m.Prism }))
);

interface MessageBubbleProps {
  message: Message;
  onRegenerate?: () => void;
  onCopy?: (content: string) => void;
}

/**
 * Link allowlist for model-rendered markdown. Model output is untrusted
 * (prompt injection can smuggle `[x](javascript:…)` through knowledge files),
 * and react-markdown passes hrefs through unsanitized — so only http/https,
 * mailto and same-app relative links survive; everything else degrades to
 * plain text (empty href renders no clickable link).
 */
// eslint-disable-next-line react/only-export-components
export function safeMarkdownUrl(url: string): string {
  const u = url.trim().toLowerCase();
  if (
    u.startsWith('http://') ||
    u.startsWith('https://') ||
    u.startsWith('mailto:') ||
    u.startsWith('/') ||
    u.startsWith('#')
  ) {
    return url;
  }
  return '';
}

const codeBlocks: Record<string, string> = {
  js: 'javascript', ts: 'typescript', jsx: 'javascript', tsx: 'typescript',
  py: 'python', rs: 'rust', go: 'go', java: 'java', cpp: 'cpp', c: 'c',
  cs: 'csharp', html: 'html', css: 'css', json: 'json', yaml: 'yaml',
  md: 'markdown', sh: 'bash', bash: 'bash', zsh: 'bash', sql: 'sql',
};

export function MessageBubble({ message, onRegenerate, onCopy }: MessageBubbleProps) {
  const [copied, setCopied] = useState(false);
  const { addToast } = useToast();

  const handleCopy = useCallback(() => {
    if (onCopy) onCopy(message.content); else navigator.clipboard.writeText(message.content);
    setCopied(true); addToast({ type: 'success', title: 'تم النسخ', message: 'تم نسخ المحتوى إلى الحافظة' }); setTimeout(() => setCopied(false), 2000);
  }, [message.content, onCopy, addToast]);

  const isUser = message.role === 'user';
  const isAssistant = message.role === 'assistant';

  const renderCodeBlock = (props: { node?: any; children?: React.ReactNode }) => {
    const codeEl = (props.children as unknown as { props?: { className?: string; children?: string } })?.props
    const language = codeEl?.className?.replace('language-', '') || ''
    const code = codeEl?.children || ''
    const lang = codeBlocks[language] || language || 'text';

    return (
      <div className="relative group my-2 rounded-lg overflow-hidden bg-gray-900 dark:bg-gray-950 border border-gray-700">
        <div className="flex items-center justify-between px-3 py-1.5 bg-gray-800 dark:bg-gray-900 border-b border-gray-700">
          <span className="text-xs text-gray-400 font-mono">{language || 'text'}</span>
          <button onClick={handleCopy} className="p-1.5 rounded text-gray-400 hover:text-white hover:bg-gray-700 transition-colors opacity-0 group-hover:opacity-100" aria-label="نسخ الكود">{copied ? <CheckCheck className="h-4 w-4 text-green-400" /> : <Copy className="h-4 w-4" />}</button>
        </div>
        <Suspense fallback={<pre className="m-0 p-4 text-sm font-mono whitespace-pre-wrap">{code}</pre>}>
          <SyntaxHighlighter language={lang} style={oneDark} customStyle={{ margin: 0, padding: '1rem', fontSize: '0.875rem', lineHeight: '1.6' }} showLineNumbers={false} wrapLines={true}>{code}</SyntaxHighlighter>
        </Suspense>
      </div>
    );
  };

  const renderInlineCode = ({ children }: { children: React.ReactNode }) => (
    <code className="bg-gray-100 dark:bg-gray-800 px-1.5 py-0.5 rounded text-sm font-mono text-primary-700 dark:text-primary-300">{children}</code>
  );

  if (message.role === 'system') {
    return (
      <div className="flex items-start gap-3 justify-start">
        <div className="flex-1 max-w-[85%] text-left">
          <div className="bg-gray-100 dark:bg-gray-800 rounded-lg p-3 text-sm text-gray-600 dark:text-gray-400">{message.content}</div>
          <div className="mt-1 flex items-center gap-2 text-xs text-gray-400 justify-start">
            <span>{formatTime(message.timestamp)}</span>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className={clsx('flex items-start gap-3 animate-in', isUser ? 'justify-end' : 'justify-start')}>
      {!isUser && <div className="flex-shrink-0 w-8 h-8 rounded-full bg-primary-100 dark:bg-primary-900/30 flex items-center justify-center"><Sparkles className="h-4 w-4 text-primary-600 dark:text-primary-400" /></div>}
      <div className={clsx('flex-1 max-w-[85%]', isUser ? 'text-right' : 'text-left')}>
        <div className={clsx('rounded-2xl p-4', isUser ? 'bg-primary-600 text-white rounded-tr-none' : 'bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100 rounded-tl-none shadow-sm border border-gray-200 dark:border-gray-700')}>
          <ReactMarkdown urlTransform={safeMarkdownUrl} components={{
            code: renderInlineCode,
            pre: renderCodeBlock,
            p: ({ children }) => <p className="whitespace-pre-wrap break-words">{children}</p>,
            ul: ({ children }) => <ul className="list-disc list-inside space-y-1 my-2">{children}</ul>,
            ol: ({ children }) => <ol className="list-decimal list-inside space-y-1 my-2">{children}</ol>,
            li: ({ children }) => <li className="ml-4">{children}</li>,
            blockquote: ({ children }) => <blockquote className="border-r-4 border-primary-500 pr-3 my-2 italic text-gray-600 dark:text-gray-400">{children}</blockquote>,
            a: ({ href, children }) => <a href={href} target="_blank" rel="noopener noreferrer" className="text-primary-600 dark:text-primary-400 underline hover:no-underline">{children}</a>,
            strong: ({ children }) => <strong className="font-semibold">{children}</strong>,
            em: ({ children }) => <em className="italic">{children}</em>
          }}>{message.content}</ReactMarkdown>

          {message.toolCalls && message.toolCalls.length > 0 && (
            <div className="mt-3 space-y-2">
              {message.toolCalls.map((tool) => (
                <div key={tool.id} className="bg-gray-100 dark:bg-gray-700 rounded-lg p-3 text-sm">
                  <div className="flex items-center gap-2 text-gray-600 dark:text-gray-400 mb-1">
                    <span className="font-mono text-xs">{tool.name}</span>
                    <span className="text-xs opacity-60">({JSON.stringify(tool.arguments).length} chars)</span>
                  </div>
                  {tool.result && (
                    <details className="mt-1">
                      <summary className="cursor-pointer text-xs text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-200">نتيجة الأداة</summary>
                      <pre className="mt-1 text-xs bg-gray-200 dark:bg-gray-900 p-2 rounded overflow-x-auto max-h-32">{tool.result}</pre>
                    </details>
                  )}
                </div>
              ))}
            </div>
          )}
        </div>

        <div className={clsx('mt-1.5 flex items-center gap-2 text-xs text-gray-400', isUser ? 'justify-end' : 'justify-start')}>
          <span>{formatTime(message.timestamp)}</span>
          {message.tokensUsed && (<><span className="flex items-center gap-1 px-2 py-0.5 rounded bg-gray-100 dark:bg-gray-800"><span className="text-xs">{message.tokensUsed}</span><span className="text-xs text-gray-500">توكنز</span></span></>)}
          {message.tokensPerSecond && message.tokensPerSecond > 0 && (<span className="flex items-center gap-1 px-2 py-0.5 rounded bg-gray-100 dark:bg-gray-800"><span className="text-xs">{message.tokensPerSecond.toFixed(1)}</span><span className="text-xs text-gray-500">t/s</span></span>)}
          {isAssistant && !message.isStreaming && onRegenerate && (<button onClick={onRegenerate} className="p-1 rounded hover:bg-gray-100 dark:hover:bg-gray-800 text-gray-400 hover:text-gray-600 dark:hover:text-gray-200 transition-colors" aria-label="إعادة التوليد"><svg className="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" /></svg></button>)}
          {isAssistant && !message.isStreaming && (<button onClick={handleCopy} className="p-1 rounded hover:bg-gray-100 dark:hover:bg-gray-800 text-gray-400 hover:text-gray-600 dark:hover:text-gray-200 transition-colors" aria-label="نسخ الرسالة">{copied ? <svg className="h-4 w-4 text-green-400" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" /></svg> : <svg className="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z" /></svg>}</button>)}
        </div>
      </div>
      {isUser && <div className="flex-shrink-0 w-8 h-8 rounded-full bg-gray-200 dark:bg-gray-700 flex items-center justify-center"><svg className="h-4 w-4 text-gray-600 dark:text-gray-300" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z" /></svg></div>}
    </div>
  );
}
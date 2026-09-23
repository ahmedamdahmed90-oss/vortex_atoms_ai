import { useState } from 'react';
import { useDashboard } from '../../hooks/useDashboard';
import { Card, CardContent } from '../ui/Card';
import { Button } from '../ui/Button';
import { Textarea } from '../ui/Input';
import { Modal } from '../ui/Modal';

export function ToolsTab() {
  const { tools, executeTool } = useDashboard();
  const [selectedTool, setSelectedTool] = useState<string | null>(null);
  const [toolArgs, setToolArgs] = useState('{}');
  const [executing, setExecuting] = useState(false);
  const [result, setResult] = useState<string | null>(null);
  const [showResultModal, setShowResultModal] = useState(false);

  const handleExecute = async () => {
    try { setExecuting(true); let args: Record<string, unknown> = {}; try { args = JSON.parse(toolArgs); } catch { throw new Error('معاملات JSON غير صالحة'); } const data = await executeTool(selectedTool ?? '', args); setResult(JSON.stringify(data, null, 2)); setShowResultModal(true); } catch (error) { setResult(`خطأ: ${error}`); setShowResultModal(true); } finally { setExecuting(false); }
  };

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between"><div><h2 className="text-xl font-semibold text-gray-900 dark:text-gray-100">الأدوات</h2><p className="text-gray-500 dark:text-gray-400">تنفيذ واختبار الأدوات المتاحة</p></div></div>

      {tools && tools.length > 0 ? (
        <div className="space-y-4">{tools.map((tool) => (<Card key={tool.name} hover><CardContent className="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4"><div className="flex items-start gap-4"><div className="p-3 rounded-xl bg-purple-100 dark:bg-purple-900/30"><svg className="h-6 w-6 text-purple-600 dark:text-purple-400" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10 17l-4-4m0 0l4 4m-4-4h18" /></svg></div><div><p className="font-semibold text-gray-900 dark:text-gray-100">{tool.name}</p><p className="text-sm text-gray-500 dark:text-gray-400">{tool.description}</p></div></div><Button onClick={() => setSelectedTool(tool.name)} leftIcon={<svg className="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10 17l-4-4m0 0l4 4m-4-4h18" /></svg>}>تنفيذ</Button></CardContent></Card>))}</div>
      ) : (
        <div className="text-center py-12"><p className="text-gray-500 dark:text-gray-400">لا توجد أدوات متاحة</p></div>
      )}

      {selectedTool && <Modal isOpen={!!selectedTool} onClose={() => { setSelectedTool(null); setToolArgs('{}'); setResult(null); }} title={`تنفيذ: ${selectedTool}`} size="lg"><div className="space-y-4"><div><label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">المعاملات (JSON)</label><Textarea value={toolArgs} onChange={(e) => setToolArgs(e.target.value)} placeholder='{"param": "value"}' rows={8} className="font-mono text-sm" /></div><div className="flex justify-end gap-3"><Button variant="secondary" onClick={() => { setSelectedTool(null); setToolArgs('{}'); }}>إلغاء</Button><Button onClick={handleExecute} loading={executing} leftIcon={<svg className="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M14 5l7 7m0 0l-7 7m7-7H3" /></svg>}>تنفيذ</Button></div></div></Modal>}

      {showResultModal && <Modal isOpen={showResultModal} onClose={() => { setShowResultModal(false); setResult(null); }} title="نتيجة التنفيذ" size="lg"><div className="max-h-96 overflow-y-auto"><pre className="p-4 bg-gray-100 dark:bg-gray-900 rounded-lg text-sm font-mono text-gray-900 dark:text-gray-100 overflow-x-auto whitespace-pre-wrap">{result}</pre></div><div className="mt-4 flex justify-end"><Button onClick={() => { setShowResultModal(false); setResult(null); }}>إغلاق</Button></div></Modal>}
    </div>
  );
}
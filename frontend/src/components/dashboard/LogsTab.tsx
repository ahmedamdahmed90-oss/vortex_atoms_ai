import { useState } from 'react';
import { useDashboard } from '../../hooks/useDashboard';
import { Card, CardHeader, CardContent, CardFooter } from '../ui/Card';
import { Button } from '../ui/Button';
import { Select } from '../ui/Select';

export function LogsTab() {
  const { logs, logFilter, setLogFilter, clearLogs } = useDashboard();
  const [autoScroll, setAutoScroll] = useState(true);

  const filteredLogs = logs.filter((log) => logFilter === 'all' || log.toLowerCase().includes(logFilter.toLowerCase()));

  const handleExport = () => {
    const blob = new Blob([filteredLogs.join('\n')], { type: 'text/plain;charset=utf-8' });
    const url = URL.createObjectURL(blob);
    const anchor = document.createElement('a');
    anchor.href = url;
    anchor.download = 'vortex-logs.txt';
    document.body.appendChild(anchor);
    anchor.click();
    anchor.remove();
    URL.revokeObjectURL(url);
  };

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between"><div><h2 className="text-xl font-semibold text-gray-900 dark:text-gray-100">السجلات</h2><p className="text-gray-500 dark:text-gray-400">عرض سجلات النظام والأخطاء</p></div><div className="flex items-center gap-3"><Select value={logFilter} onChange={(e) => setLogFilter(e.target.value as 'all' | 'error' | 'warn' | 'info' | 'debug')} options={[{ value: 'all', label: 'الكل' },{ value: 'error', label: 'خطأ' },{ value: 'warn', label: 'تحذير' },{ value: 'info', label: 'معلومات' },{ value: 'debug', label: 'تصحيح' }]} className="w-40" /><Button variant="secondary" onClick={clearLogs} leftIcon={<svg className="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" /></svg>}>مسح</Button></div></div>

      <Card>
        <CardHeader title={`السجلات (${filteredLogs.length})`} action={<label className="flex items-center gap-2 text-sm text-gray-600 dark:text-gray-400 cursor-pointer"><input type="checkbox" checked={autoScroll} onChange={(e) => setAutoScroll(e.target.checked)} className="w-4 h-4 rounded border-gray-300 text-primary-600 focus:ring-primary-500" />تمرير تلقائي</label>} />
        <CardContent className="p-0"><div className="max-h-[500px] overflow-y-auto font-mono text-sm">{filteredLogs.length === 0 ? (<div className="p-8 text-center text-gray-500 dark:text-gray-400"><p>لا توجد سجلات</p></div>) : (<div className="p-4 space-y-1">{filteredLogs.map((log, index) => (<div key={index} className="px-3 py-1.5 rounded border-l-4 transition-colors" style={{ borderLeftColor: log.includes('ERROR') || log.includes('error') ? '#ef4444' : log.includes('WARN') || log.includes('warn') ? '#f59e0b' : log.includes('INFO') || log.includes('info') ? '#3b82f6' : '#6b7280', backgroundColor: log.includes('ERROR') || log.includes('error') ? '#fef2f2' : log.includes('WARN') || log.includes('warn') ? '#fffbeb' : log.includes('INFO') || log.includes('info') ? '#eff6ff' : '#f9fafb' }}><pre className="whitespace-pre-wrap text-sm leading-relaxed">{log}</pre></div>))}</div>)}</div></CardContent>
        <CardFooter><div className="flex items-center justify-between text-sm text-gray-500 dark:text-gray-400"><span>إجمالي: {logs.length} | مصفى: {filteredLogs.length}</span><Button variant="ghost" size="sm" onClick={handleExport} leftIcon={<svg className="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-8l-4-4m0 0L8 8m4-4v12" /></svg>}>تصدير</Button></div></CardFooter>
      </Card>
    </div>
  );
}
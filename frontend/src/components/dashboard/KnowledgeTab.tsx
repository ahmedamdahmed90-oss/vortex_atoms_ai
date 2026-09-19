import { useState } from 'react';
import { useDashboard } from '../../hooks/useDashboard';
import { Card, CardHeader, CardContent } from '../ui/Card';
import { Button } from '../ui/Button';
import { Textarea } from '../ui/Input';
import { Modal } from '../ui/Modal';
import { useToast } from '../../components/ui/Toast';

export function KnowledgeTab() {
  const { knowledgeChunks, searchResults, searchKnowledge, importKnowledge } = useDashboard();
  const { addToast: _addToast } = useToast();
  const [searchQuery, setSearchQuery] = useState('');
  const [showImportModal, setShowImportModal] = useState(false);
  const [importText, setImportText] = useState('');
  const [importing, setImporting] = useState(false);
  const [searching, _setSearching] = useState(false);

  const handleSearch = () => {
    if (searchQuery.trim()) searchKnowledge(searchQuery);
  };

  const handleImport = async () => {
    try { setImporting(true); await importKnowledge(importText); setImportText(''); setShowImportModal(false); } catch (error) { console.error('Import failed:', error); } finally { setImporting(false); }
  };

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between"><div><h2 className="text-xl font-semibold text-gray-900 dark:text-gray-100">قاعدة المعرفة</h2><p className="text-gray-500 dark:text-gray-400">استيراد وبحث في المعرفة</p></div><Button onClick={() => setShowImportModal(true)} leftIcon={<svg className="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-8l-4-4m0 0L8 8m4-4v12" /></svg>}>استيراد معرفة</Button></div>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <Card>
          <CardHeader title="البحث في المعرفة" description="ابحث عن معلومات في قاعدة المعرفة" />
          <CardContent>
            <div className="space-y-4">
              <div className="flex gap-2"><input type="text" value={searchQuery} onChange={(e) => setSearchQuery(e.target.value)} onKeyDown={(e) => e.key === 'Enter' && handleSearch()} placeholder="ابحث في المعرفة..." className="flex-1 px-4 py-2.5 rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-primary-500" /><Button onClick={handleSearch} loading={searching} leftIcon={<svg className="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" /></svg>}>بحث</Button></div>

              {searchResults && searchResults.length > 0 ? (
                <div className="space-y-3 max-h-96 overflow-y-auto">{searchResults.map((result, index) => (<div key={result.id} className="p-4 rounded-lg border border-gray-200 dark:border-gray-700 hover:bg-gray-50 dark:hover:bg-gray-800"><div className="flex items-start justify-between gap-3"><div className="flex-1"><div className="flex items-center gap-2 mb-2"><span className="text-sm font-medium text-gray-900 dark:text-gray-100">نتيجة #{index + 1}</span><span className="px-2 py-0.5 rounded text-xs bg-primary-100 dark:bg-primary-900/30 text-primary-700 dark:text-primary-300">{result.score.toFixed(3)}</span></div><p className="text-gray-700 dark:text-gray-300 text-sm line-clamp-3">{result.text}</p></div></div></div>))}</div>
              ) : searchResults && searchResults.length === 0 ? (
                <div className="text-center py-8 text-gray-500 dark:text-gray-400"><p>لا توجد نتائج للبحث</p></div>
              ) : (
                <div className="text-center py-8 text-gray-400"><p className="text-sm">أدخل استعلام بحث لعرض النتائج</p></div>
              )}
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader title="معلومات قاعدة المعرفة" />
          <CardContent>
            <div className="space-y-4">
              <div className="flex items-center justify-between p-4 rounded-lg bg-gray-50 dark:bg-gray-800"><div className="flex items-center gap-3"><div className="p-3 rounded-xl bg-blue-100 dark:bg-blue-900/30"><svg className="h-6 w-6 text-blue-600 dark:text-blue-400" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 7v10c0 2.2 1.8 4 4 4h12c2.2 0 4-1.8 4-4V7" /></svg></div><div><p className="text-sm text-gray-500 dark:text-gray-400">إجمالي القطع</p><p className="text-2xl font-bold text-gray-900 dark:text-gray-100">{knowledgeChunks}</p></div></div></div>
              <div className="grid grid-cols-2 gap-4"><div className="p-4 rounded-lg bg-gray-50 dark:bg-gray-800"><p className="text-sm text-gray-500 dark:text-gray-400">الحالة</p><p className="font-medium text-gray-900 dark:text-gray-100">جاهزة</p></div><div className="p-4 rounded-lg bg-gray-50 dark:bg-gray-800"><p className="text-sm text-gray-500 dark:text-gray-400">آخر تحديث</p><p className="font-medium text-gray-900 dark:text-gray-100">الآن</p></div></div>
            </div>
          </CardContent>
          <div className="p-4 border-t border-gray-200 dark:border-gray-700"><Button variant="secondary" leftIcon={<svg className="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" /></svg>}>تحديث</Button></div>
        </Card>
      </div>

      <Modal isOpen={showImportModal} onClose={() => setShowImportModal(false)} title="استيراد معرفة جديدة" description="أضف نصاً جديداً إلى قاعدة المعرفة"><div className="space-y-4"><Textarea value={importText} onChange={(e) => setImportText(e.target.value)} placeholder="أدخل النص الذي تريد إضافته إلى قاعدة المعرفة..." rows={6} label="النص" /><div className="text-sm text-gray-500 dark:text-gray-400">سيتم تقسيم النص تلقائياً إلى قطع وفهرسته للبحث.</div></div><div className="mt-6 flex justify-end gap-3"><Button variant="secondary" onClick={() => setShowImportModal(false)}>إلغاء</Button><Button onClick={handleImport} loading={importing} disabled={!importText.trim()}>استيراد</Button></div></Modal>
    </div>
  );
}
import { clsx } from 'clsx';
import { useState } from 'react';
import { useDashboard } from '../../hooks/useDashboard';
import { Card, CardHeader, CardContent } from '../ui/Card';
import { Button } from '../ui/Button';
import { Spinner } from '../ui/Spinner';
import { Modal } from '../ui/Modal';
import { useToast } from '../../components/ui/Toast';

export function ModelsTab() {
  const { models, currentModel, loading, swapModel, fetchModels } = useDashboard();
  const { addToast } = useToast();
  const [showSwapModal, setShowSwapModal] = useState(false);
  const [selectedModel, setSelectedModel] = useState('');
  const [swapping, setSwapping] = useState(false);

  const modelList = Array.isArray(models) ? models : (models ? [models] : []);

  const handleSwap = async () => {
    if (!selectedModel) return;
    setSwapping(true);
    try {
      await swapModel(selectedModel);
      addToast({ type: 'success', title: 'تم التبديل', message: `تم التبديل إلى ${selectedModel}` });
      setShowSwapModal(false);
    } catch (error) {
      addToast({ type: 'error', title: 'فشل التبديل', message: String(error) });
    } finally {
      setSwapping(false);
    }
  };

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-xl font-semibold text-gray-900 dark:text-gray-100">إدارة النماذج</h2>
          <p className="text-gray-500 dark:text-gray-400">عرض وتبديل النماذج المحملة</p>
        </div>
        <Button onClick={fetchModels} loading={loading} leftIcon={<svg className="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" /></svg>}>تحديث</Button>
      </div>

      <Card>
        <CardHeader title="النموذج الحالي" />
        <CardContent>
          {currentModel ? (
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-4">
                <div className="p-3 rounded-xl bg-primary-100 dark:bg-primary-900/30"><svg className="h-6 w-6 text-primary-600 dark:text-primary-400" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10 20l4-16m4 4l4 4-4 4M6 16l-4-4 4-4" /></svg></div>
                <div><p className="text-lg font-semibold text-gray-900 dark:text-gray-100">{currentModel}</p><p className="text-sm text-gray-500 dark:text-gray-400">نموذج نشط</p></div>
              </div>
              <Button variant="secondary" onClick={() => { setSelectedModel(''); setShowSwapModal(true); }} leftIcon={<svg className="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8 7h12m0 0l-4-4m4 4l-4 4m0 6H4m0 0l4 4m-4-4l4-4" /></svg>}>تبديل النموذج</Button>
            </div>
          ) : (
            <div className="flex items-center gap-4 text-gray-500 dark:text-gray-400"><Spinner className="h-6 w-6" /><span>جاري تحميل معلومات النموذج...</span></div>
          )}
        </CardContent>
      </Card>

      <Card>
        <CardHeader title="النماذج المتاحة" description="قائمة النماذج المتاحة للتبديل" />
        <CardContent>
{modelList.length > 0 ? (
            <div className="space-y-3">
              {modelList.map((model) => (
                <div key={model.name} className="flex items-center justify-between p-4 rounded-lg border border-gray-200 dark:border-gray-700 hover:bg-gray-50 dark:hover:bg-gray-800 transition-colors">
                  <div className="flex items-center gap-4">
                    <div className="p-2 rounded-lg bg-gray-100 dark:bg-gray-800"><svg className="h-5 w-5 text-gray-600 dark:text-gray-400" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10 20l4-16m4 4l4 4-4 4M6 16l-4-4 4-4" /></svg></div>
                    <div><p className="font-medium text-gray-900 dark:text-gray-100">{model.name}</p><p className="text-sm text-gray-500 dark:text-gray-400">{model.architecture} • {model.max_seq_len?.toLocaleString() ?? ''} seq • {model.max_generation_tokens?.toLocaleString() ?? ''} gen</p></div>
                  </div>
                  <div className="flex items-center gap-2">
                    {currentModel === model.name && <span className="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-400">نشط</span>}
                    <Button variant="ghost" size="sm" onClick={() => { setSelectedModel(model.name); setShowSwapModal(true); }} leftIcon={<svg className="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8 7h12m0 0l-4-4m4 4l-4 4m0 6H4m0 0l4 4m-4-4l4-4" /></svg>}>تبديل</Button>
                  </div>
                </div>
              ))}
            </div>
          ) : (
            <div className="text-center py-8 text-gray-500 dark:text-gray-400"><svg className="h-12 w-12 mx-auto mb-4 opacity-50" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10 20l4-16m4 4l4 4-4 4M6 16l-4-4 4-4" /></svg><p>لا توجد نماذج متاحة</p></div>
          )}
        </CardContent>
      </Card>

      <Modal isOpen={showSwapModal} onClose={() => setShowSwapModal(false)} title="تبديل النموذج" description="اختر النموذج للتبديل إليه">
        <div className="space-y-4">
          {modelList.map((model) => (
            <button key={model.name} onClick={() => setSelectedModel(model.name)} className={clsx('w-full p-4 rounded-lg border-2 text-right transition-all', selectedModel === model.name ? 'border-primary-500 bg-primary-50 dark:bg-primary-900/20' : 'border-gray-200 dark:border-gray-700 hover:border-primary-300 dark:hover:border-primary-700')}>
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-3"><div className={clsx('p-2 rounded-lg', selectedModel === model.name ? 'bg-primary-100 dark:bg-primary-900/30' : 'bg-gray-100 dark:bg-gray-800')}>
                  <svg className={clsx('h-5 w-5', selectedModel === model.name ? 'text-primary-600 dark:text-primary-400' : 'text-gray-600 dark:text-gray-400')} fill="none" viewBox="0 0 24 24" stroke="currentColor"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10 20l4-16m4 4l4 4-4 4M6 16l-4-4 4-4" /></svg></div><div><p className="font-medium text-gray-900 dark:text-gray-100">{model.name}</p><p className="text-sm text-gray-500 dark:text-gray-400">{model.architecture}</p></div></div>
                {selectedModel === model.name && <svg className="h-5 w-5 text-primary-600 dark:text-primary-400" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" /></svg>}
              </div>
            </button>
          ))}
        </div>
        <div className="mt-6 flex justify-end gap-3"><Button variant="secondary" onClick={() => setShowSwapModal(false)}>إلغاء</Button><Button onClick={handleSwap} loading={swapping} disabled={!selectedModel}>{swapping ? 'جاري التبديل...' : 'تبديل النموذج'}</Button></div>
      </Modal>
    </div>
  );
}